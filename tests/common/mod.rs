//! Shared test harness for integration tests.
//!
//! Each test scenario file (tests/*.rs) is its own binary; declare
//! `mod common;` at the top of the scenario file to pull this in.
//!
//! Per D-02, D-04.

#![allow(dead_code)] // each test binary uses a subset

use std::path::PathBuf;
use wiremock::MockServer;
use fastmail_cli::jmap::JmapClient;
use fastmail_cli::carddav::CardDavClient;
use fastmail_cli::caldav::CalDavClient;

/// Spawn a fresh wiremock server bound to 127.0.0.1:random_port.
pub async fn start_mock_server() -> MockServer {
    MockServer::start().await
}

/// Read a fixture file from tests/fixtures/. Panics on failure (test-only).
pub fn load_fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to load fixture {}: {}", path.display(), e))
}

/// Build the JMAP session response JSON with {{BASE_URL}} placeholders replaced.
pub fn jmap_session_response(base_url: &str) -> String {
    load_fixture("jmap_session.json").replace("{{BASE_URL}}", base_url)
}

/// Build a JmapClient pointing at the given mock server's /jmap/session endpoint.
pub fn test_jmap_client(server: &MockServer) -> JmapClient {
    let session_url = format!("{}/jmap/session", server.uri());
    JmapClient::new_with_session_url("test-token".to_string(), session_url)
        .expect("test jmap client")
}

pub fn test_carddav_client(server: &MockServer) -> CardDavClient {
    CardDavClient::new_with_base_url(
        "test@example.com".to_string(),
        "test-password".to_string(),
        server.uri(),
    )
    .expect("test carddav client")
}

pub fn test_caldav_client(server: &MockServer) -> CalDavClient {
    CalDavClient::new_with_base_url(
        "test@example.com".to_string(),
        "test-password".to_string(),
        server.uri(),
    )
    .expect("test caldav client")
}
