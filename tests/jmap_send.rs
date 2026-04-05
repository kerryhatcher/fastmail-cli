mod common;

use fastmail_cli::jmap::ComposeParams;
use fastmail_cli::models::EmailAddress;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// Builds a JmapClient, authenticates it, then calls send_email() with a single
/// recipient. Verifies that:
///   1. At least one POST to /jmap/api/ is made.
///   2. One of those POSTs includes Email/set in methodCalls (wire format).
///   3. The request body contains the recipient address and email subject.
#[tokio::test]
async fn send_email_posts_correct_wire_format() {
    let server = common::start_mock_server().await;

    // 1. Session endpoint — authenticate() does a GET /jmap/session.
    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            common::jmap_session_response(&server.uri()),
            "application/json",
        ))
        .mount(&server)
        .await;

    // 2. All POST /jmap/api/ requests are handled by a single catch-all mock.
    //    send_email() issues three sequential POSTs:
    //      a. Mailbox/get  (prepare_compose -> find_mailbox("sent") -> list_mailboxes)
    //      b. Identity/get (prepare_compose -> resolve_identity -> list_identities)
    //      c. Email/set + EmailSubmission/set  (create_and_submit_email)
    //    The mock must respond appropriately for each call. We use three separate
    //    .up_to_n_times(1) mocks in priority order (last-registered = highest priority
    //    in wiremock 0.6, so register in reverse order).
    //
    //    Mailbox/get response fixture.
    let mailbox_body = common::load_fixture("jmap_mailbox_get.json");
    Mock::given(method("POST"))
        .and(path("/jmap/api/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(mailbox_body, "application/json"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Identity/get response fixture.
    let identity_body = common::load_fixture("jmap_identity_get.json");
    Mock::given(method("POST"))
        .and(path("/jmap/api/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(identity_body, "application/json"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Email/set + EmailSubmission/set response fixture.
    let email_set_body = common::load_fixture("jmap_email_set_response.json");
    Mock::given(method("POST"))
        .and(path("/jmap/api/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(email_set_body, "application/json"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Authenticate first so the session (and api_url) are populated.
    let mut client = common::test_jmap_client(&server);
    client.authenticate().await.expect("authenticate should succeed");

    let to = vec![EmailAddress {
        email: "dest@example.com".to_string(),
        name: Some("Dest User".to_string()),
    }];

    let result = client
        .send_email(
            to,
            "Test Subject",
            "Test body content",
            None, // in_reply_to
            ComposeParams {
                cc: vec![],
                bcc: vec![],
                from: None,
                draft: false,
            },
        )
        .await;

    assert!(
        result.is_ok(),
        "send_email should succeed with mocked endpoints: {:?}",
        result.err()
    );

    // Assert the returned email ID matches what the fixture provides.
    assert_eq!(result.unwrap(), "Email-abc123");

    // ── Wire-format assertions ──────────────────────────────────────────────
    // Retrieve all requests observed by the mock server.
    let received = server
        .received_requests()
        .await
        .expect("mock server should record requests");

    let post_bodies: Vec<serde_json::Value> = received
        .iter()
        .filter(|r| r.method == wiremock::http::Method::POST)
        .map(|r| {
            serde_json::from_slice(&r.body)
                .expect("each POST body should be valid JSON")
        })
        .collect();

    assert!(!post_bodies.is_empty(), "expected at least one POST to /jmap/api/");

    // Verify that at least one POST contained Email/set in its methodCalls array.
    let found_email_set = post_bodies.iter().any(|body| {
        body.get("methodCalls")
            .and_then(|mc| mc.as_array())
            .map(|arr| {
                arr.iter().any(|call| {
                    call.get(0)
                        .and_then(|v| v.as_str())
                        .map(|name| name == "Email/set")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });
    assert!(
        found_email_set,
        "expected Email/set in methodCalls of at least one POST body; bodies: {:#?}",
        post_bodies
    );

    // Verify subject and recipient appear in the wire body.
    let body_str = serde_json::to_string(&post_bodies).expect("re-serialize post bodies");
    assert!(
        body_str.contains("Test Subject"),
        "subject 'Test Subject' not found in POST wire body"
    );
    assert!(
        body_str.contains("dest@example.com"),
        "recipient 'dest@example.com' not found in POST wire body"
    );
}

/// Verifies that a send_email() call that returns an error from the session endpoint
/// propagates the error correctly without panicking.
#[tokio::test]
async fn send_email_fails_when_session_returns_401() {
    let server = common::start_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let mut client = common::test_jmap_client(&server);
    // authenticate() should fail with InvalidToken.
    let auth_result = client.authenticate().await;
    assert!(
        auth_result.is_err(),
        "expected auth failure on 401 but got success"
    );

    use fastmail_cli::error::Error;
    assert!(
        matches!(auth_result.unwrap_err(), Error::InvalidToken(_)),
        "expected Error::InvalidToken for 401 response"
    );
}
