mod common;

use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use fastmail_cli::caldav::EventQuery;

/// CalDAV concurrent partial-failure tolerance test (D-08 coverage 4).
///
/// 3 calendars (Default, Personal, Work) are discovered via PROPFIND.
/// The middle calendar (Personal, alphabetically second) returns HTTP 500 on REPORT.
/// list_events should succeed and return events from the 2 working calendars.
#[tokio::test]
async fn list_events_tolerates_single_failing_calendar() {
    let server = common::start_mock_server().await;

    // Principal discovery: calendar-home-set href must be a path, not a full URL,
    // because the client prepends self.base_url when building the PROPFIND request URL.
    let principal_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:href>/dav/principals/user/test@example.com/</d:href>
    <d:propstat>
      <d:prop>
        <c:calendar-home-set><d:href>/dav/calendars/user/test@example.com/</d:href></c:calendar-home-set>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
    Mock::given(method("PROPFIND"))
        .and(path("/dav/principals/user/test@example.com/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_raw(principal_xml.to_string(), "application/xml"),
        )
        .mount(&server)
        .await;

    // Calendar list PROPFIND: fixture contains 3 calendars — Default, Work, Personal.
    let calendars_xml = common::load_fixture("caldav_calendars_propfind.xml");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/calendars/user/test@example.com/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_raw(calendars_xml, "application/xml"),
        )
        .mount(&server)
        .await;

    // Default REPORT -> success with 1 event
    let multiget = common::load_fixture("caldav_calendar_multiget.xml");
    Mock::given(method("REPORT"))
        .and(path("/dav/calendars/user/test@example.com/Default/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_raw(multiget.clone(), "application/xml"),
        )
        .mount(&server)
        .await;

    // Personal REPORT -> 500 (failing calendar — middle in sorted order: Default, Personal, Work)
    Mock::given(method("REPORT"))
        .and(path("/dav/calendars/user/test@example.com/Personal/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    // Work REPORT -> success with 1 event
    Mock::given(method("REPORT"))
        .and(path("/dav/calendars/user/test@example.com/Work/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_raw(multiget, "application/xml"),
        )
        .mount(&server)
        .await;

    let client = common::test_caldav_client(&server);

    // list_events with no date range and no calendar_id -> fans out across all calendars.
    let events = client
        .list_events(EventQuery {
            calendar_id: None,
            start: None,
            end: None,
        })
        .await
        .expect("list_events should succeed even when one calendar returns 500");

    // 2 working calendars (Default + Work) each return 1 event from the fixture.
    assert!(
        events.len() >= 2,
        "expected events from 2 working calendars, got {}",
        events.len()
    );

    // Verify REPORT was issued to all 3 calendars (partial failure does not short-circuit).
    let received = server
        .received_requests()
        .await
        .expect("received requests");
    let report_count = received
        .iter()
        .filter(|r| r.method.as_str() == "REPORT")
        .count();
    assert_eq!(
        report_count, 3,
        "expected REPORTs to all 3 calendars, got {report_count}"
    );
}
