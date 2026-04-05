//! Integration tests for MCP GraphQL resolvers.
//!
//! Builds the FastmailSchema with a test AppContext (deterministic HMAC key),
//! wires a wiremock-backed JmapClient, executes queries and asserts valid output.
//! Per D-08 coverage 6, D-10: no wiremock inside src/.

mod common;

use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use fastmail_cli::mcp::graphql::{build_schema, AppContext};

/// Mailbox/get response fixture — one Inbox mailbox.
fn mailbox_get_response() -> &'static str {
    r#"{
      "methodResponses":[
        ["Mailbox/get",{
          "accountId":"u1234abcd",
          "state":"1",
          "list":[{
            "id":"Mbox-inbox",
            "name":"Inbox",
            "role":"inbox",
            "parentId":null,
            "sortOrder":0,
            "totalEmails":3,
            "unreadEmails":1,
            "totalThreads":3,
            "unreadThreads":1,
            "myRights":{
              "mayReadItems":true,"mayAddItems":true,"mayRemoveItems":true,
              "maySetSeen":true,"maySetKeywords":true,"mayCreateChild":true,
              "mayRename":true,"mayDelete":false,"maySubmit":true
            },
            "isSubscribed":true
          }],
          "notFound":[]
        },"c1"]
      ],
      "sessionState":"abc123"
    }"#
}

/// Mount JMAP session mock and Mailbox/get mock, return authenticated JmapClient.
async fn setup_jmap_client(server: &wiremock::MockServer) -> fastmail_cli::jmap::JmapClient {
    // JMAP session endpoint
    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(
                common::jmap_session_response(&server.uri()),
                "application/json",
            ),
        )
        .mount(server)
        .await;

    // JMAP API POST — respond to any method call with mailbox list
    Mock::given(method("POST"))
        .and(path("/jmap/api/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(mailbox_get_response(), "application/json"),
        )
        .mount(server)
        .await;

    let mut jmap_client = common::test_jmap_client(server);
    jmap_client.authenticate().await.expect("authenticate");
    jmap_client
}

/// JMAP-backed resolver: mailboxes query resolves through wiremock and
/// returns the Inbox mailbox in the response.
#[tokio::test]
async fn graphql_mailboxes_query_resolves_via_wiremock() {
    let server = common::start_mock_server().await;
    let jmap_client = setup_jmap_client(&server).await;
    let jmap = Arc::new(Mutex::new(jmap_client));

    // Test AppContext with deterministic zero HMAC key.
    let ctx = AppContext::new_with_key(Some(jmap), [0u8; 32]);
    let schema = build_schema(ctx);

    let query = r#"{ mailboxes { id name role } }"#;
    let resp = schema.execute(query).await;

    assert!(
        resp.errors.is_empty(),
        "unexpected GraphQL errors: {:?}",
        resp.errors
    );

    let json = serde_json::to_value(&resp.data).expect("serialize data");
    let json_str = json.to_string();
    assert!(
        json_str.contains("Inbox"),
        "expected Inbox mailbox in response, got: {json_str}"
    );
    assert!(
        json_str.contains("inbox"),
        "expected inbox role in response, got: {json_str}"
    );
}

/// Non-JMAP test: schema introspection works without any HTTP calls.
#[tokio::test]
async fn graphql_introspection_works_without_jmap() {
    let ctx = AppContext::new_with_key(None, [0u8; 32]);
    let schema = build_schema(ctx);

    let resp = schema
        .execute("{ __schema { queryType { name } } }")
        .await;

    assert!(
        resp.errors.is_empty(),
        "introspection errors: {:?}",
        resp.errors
    );
    let json = serde_json::to_value(&resp.data).unwrap();
    assert!(
        json.to_string().contains("queryType"),
        "expected queryType in introspection response"
    );
}

/// Confirmation token determinism: same key + same inputs always produce the same token.
#[tokio::test]
async fn confirmation_token_is_deterministic_with_same_key() {
    let token1 =
        AppContext::new_with_key(None, [0u8; 32]).confirmation_token(&["deleteContact", "abc"]);
    let token2 =
        AppContext::new_with_key(None, [0u8; 32]).confirmation_token(&["deleteContact", "abc"]);
    assert_eq!(
        token1, token2,
        "same key + same inputs must produce the same token"
    );
}

/// Confirmation token isolation: different keys produce different tokens.
#[tokio::test]
async fn confirmation_token_differs_across_keys() {
    let token_a =
        AppContext::new_with_key(None, [0u8; 32]).confirmation_token(&["deleteContact", "abc"]);
    let token_b =
        AppContext::new_with_key(None, [1u8; 32]).confirmation_token(&["deleteContact", "abc"]);
    assert_ne!(
        token_a, token_b,
        "different keys must produce different tokens for the same inputs"
    );
}

/// Mailboxes query without JMAP context returns an authentication error.
#[tokio::test]
async fn graphql_mailboxes_without_jmap_returns_auth_error() {
    let ctx = AppContext::new_with_key(None, [0u8; 32]);
    let schema = build_schema(ctx);

    let resp = schema.execute("{ mailboxes { id name } }").await;

    // The resolver returns an error when no JMAP client is present.
    assert!(
        !resp.errors.is_empty(),
        "expected auth error when JMAP client absent"
    );
    let error_msg = resp.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("not authenticated") || error_msg.contains("auth"),
        "expected authentication error message, got: {}",
        resp.errors[0].message
    );
}
