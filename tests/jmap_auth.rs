mod common;

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

/// Verifies that a 200 response from /jmap/session is fully parsed into a Session:
/// username, capabilities map, and primary account ID.
#[tokio::test]
async fn authenticate_returns_session_on_200() {
    let server = common::start_mock_server().await;
    let body = common::jmap_session_response(&server.uri());

    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(body, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut client = common::test_jmap_client(&server);
    let session = client
        .authenticate()
        .await
        .expect("authenticate should succeed on 200");

    assert_eq!(session.username, "test@example.com");

    // Capabilities map should contain the expected JMAP capability URNs.
    assert!(
        session
            .capabilities
            .contains_key("urn:ietf:params:jmap:core"),
        "expected urn:ietf:params:jmap:core in capabilities"
    );
    assert!(
        session
            .capabilities
            .contains_key("urn:ietf:params:jmap:mail"),
        "expected urn:ietf:params:jmap:mail in capabilities"
    );
    assert!(
        session
            .capabilities
            .contains_key("urn:ietf:params:jmap:submission"),
        "expected urn:ietf:params:jmap:submission in capabilities"
    );

    // Primary account ID should be resolvable from the session.
    let account_id = session
        .primary_account_id()
        .expect("primary account id should be present");
    assert_eq!(account_id, "u1234abcd");
}

/// Verifies that the Bearer token from the client is forwarded in the Authorization header.
/// Uses an exact match on the header value to catch token serialization regressions.
#[tokio::test]
async fn authenticate_sends_bearer_token() {
    let server = common::start_mock_server().await;
    let body = common::jmap_session_response(&server.uri());

    // Only match when the correct token is present — any other token would
    // fall through to no mock and produce a connection error.
    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(body, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut client = common::test_jmap_client(&server);
    // If the header is wrong the mock won't match, the test will time out /
    // get an empty 404, and the assertion below catches the failure.
    let result = client.authenticate().await;
    assert!(result.is_ok(), "expected ok when bearer token matches: {:?}", result.err());
}

/// Verifies that the api_url from the session fixture matches the expected pattern
/// so downstream JMAP requests will POST to the correct endpoint.
#[tokio::test]
async fn authenticate_parses_api_url() {
    let server = common::start_mock_server().await;
    let body = common::jmap_session_response(&server.uri());

    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(body, "application/json"),
        )
        .mount(&server)
        .await;

    let mut client = common::test_jmap_client(&server);
    let session = client.authenticate().await.expect("auth ok");

    // api_url must include the mock server base so subsequent requests route correctly.
    assert!(
        session.api_url.starts_with(&server.uri()),
        "api_url '{}' should start with server uri '{}'",
        session.api_url,
        server.uri()
    );
    assert!(
        session.api_url.contains("/jmap/api/"),
        "api_url '{}' should contain /jmap/api/",
        session.api_url
    );
}
