mod common;

use fastmail_cli::error::Error;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// Helper: mount a single /jmap/session mock returning `status`, call authenticate(), and
/// return the Error. Panics if authenticate() unexpectedly succeeds.
async fn authenticate_with_status(status: u16, body: &str) -> Error {
    let server = common::start_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/jmap/session"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;

    let mut client = common::test_jmap_client(&server);
    client
        .authenticate()
        .await
        .expect_err("authenticate must fail on non-2xx status")
}

// ── 401 ─────────────────────────────────────────────────────────────────────

/// HTTP 401 from the session endpoint maps to Error::InvalidToken.
#[tokio::test]
async fn error_401_returns_invalid_token() {
    let err = authenticate_with_status(401, "unauthorized").await;
    assert!(
        matches!(err, Error::InvalidToken(_)),
        "expected Error::InvalidToken, got {err:?}"
    );
}

/// The InvalidToken message should contain a human-readable description.
#[tokio::test]
async fn error_401_invalid_token_has_message() {
    let err = authenticate_with_status(401, "unauthorized").await;
    if let Error::InvalidToken(msg) = &err {
        assert!(!msg.is_empty(), "InvalidToken message must not be empty");
    } else {
        panic!("expected Error::InvalidToken, got {err:?}");
    }
}

// ── 429 ─────────────────────────────────────────────────────────────────────

/// HTTP 429 from the session endpoint maps to Error::RateLimited.
#[tokio::test]
async fn error_429_returns_rate_limited() {
    let err = authenticate_with_status(429, "slow down").await;
    assert!(
        matches!(err, Error::RateLimited),
        "expected Error::RateLimited, got {err:?}"
    );
}

// ── 500 ─────────────────────────────────────────────────────────────────────

/// HTTP 500 from the session endpoint maps to Error::Server with the status code in the message.
#[tokio::test]
async fn error_500_returns_server() {
    let err = authenticate_with_status(500, "internal server error").await;
    match err {
        Error::Server(msg) => assert!(
            msg.contains("500"),
            "Server error message should contain '500', got: {msg}"
        ),
        other => panic!("expected Error::Server, got {other:?}"),
    }
}

/// HTTP 503 (also 5xx) should also map to Error::Server.
#[tokio::test]
async fn error_503_returns_server() {
    let err = authenticate_with_status(503, "service unavailable").await;
    assert!(
        matches!(err, Error::Server(_)),
        "expected Error::Server for 503, got {err:?}"
    );
}

// ── 4xx catch-all ────────────────────────────────────────────────────────────

/// HTTP 403 maps to Error::Server and the message includes the HTTP status code.
#[tokio::test]
async fn error_403_returns_server_with_http_code() {
    let err = authenticate_with_status(403, "forbidden").await;
    match err {
        Error::Server(msg) => assert!(
            msg.contains("403"),
            "Server error message should contain '403', got: {msg}"
        ),
        other => panic!("expected Error::Server, got {other:?}"),
    }
}

/// HTTP 400 maps to Error::Server and the message includes the HTTP status code.
#[tokio::test]
async fn error_400_returns_server_with_http_code() {
    let err = authenticate_with_status(400, "bad request").await;
    match err {
        Error::Server(msg) => assert!(
            msg.contains("400"),
            "Server error message should contain '400', got: {msg}"
        ),
        other => panic!("expected Error::Server, got {other:?}"),
    }
}

/// HTTP 422 (another 4xx) maps to Error::Server with the code in the message.
#[tokio::test]
async fn error_422_returns_server_with_http_code() {
    let err = authenticate_with_status(422, "unprocessable entity").await;
    match err {
        Error::Server(msg) => assert!(
            msg.contains("422"),
            "Server error message should contain '422', got: {msg}"
        ),
        other => panic!("expected Error::Server, got {other:?}"),
    }
}
