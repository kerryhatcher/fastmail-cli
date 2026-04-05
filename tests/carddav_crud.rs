mod common;

use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use fastmail_cli::carddav::{Contact, ContactEmail};

/// Construct a sample contact with a stable ID for predictable test URLs.
fn sample_contact() -> Contact {
    Contact {
        id: "contact-1".to_string(),
        name: "Alice Example".to_string(),
        emails: vec![ContactEmail {
            email: "alice@example.com".to_string(),
            label: Some("WORK".to_string()),
        }],
        phones: vec![],
        organization: None,
        title: None,
        notes: None,
        address: None,
        href: None,
        etag: None,
    }
}

/// CardDAV CRUD round-trip test (D-08 coverage 5).
///
/// Exercises the full wire-protocol sequence against wiremock:
///   1. PUT create  (If-None-Match: *) -> 201 Created + ETag + Location
///   2. PUT update  (If-Match: etag)   -> 204 No Content
///   3. DELETE      (If-Match: etag)   -> 204 No Content
///
/// Verifies request counts, HTTP method, and the If-None-Match header on create.
#[tokio::test]
async fn carddav_crud_roundtrip() {
    let server = common::start_mock_server().await;
    let addressbook_href = "/dav/addressbooks/user/test@example.com/Default/";
    let contact_vcf_path = "/dav/addressbooks/user/test@example.com/Default/contact-1.vcf";

    // 1. Create: PUT with If-None-Match: * -> 201 Created + ETag + Location
    Mock::given(method("PUT"))
        .and(path(contact_vcf_path))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("ETag", "\"etag-1\"")
                .insert_header("Location", contact_vcf_path),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2. Update: PUT with If-Match -> 204 No Content
    Mock::given(method("PUT"))
        .and(path(contact_vcf_path))
        .respond_with(
            ResponseTemplate::new(204).insert_header("ETag", "\"etag-2\""),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 3. Delete: DELETE -> 204 No Content
    Mock::given(method("DELETE"))
        .and(path(contact_vcf_path))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = common::test_carddav_client(&server);
    let contact = sample_contact();

    // CREATE
    let created = client
        .create_contact(addressbook_href, &contact)
        .await
        .expect("create_contact should succeed");

    assert!(
        created.href.contains("contact-1.vcf"),
        "created href should reference the vcf path, got: {}",
        created.href
    );
    assert_eq!(
        created.etag.as_deref(),
        Some("\"etag-1\""),
        "create should capture ETag from 201 response"
    );

    // UPDATE
    let created_href = created.href.clone();
    let created_etag = created.etag.clone().unwrap();
    let mut updated_contact = contact.clone();
    updated_contact.name = "Alice Updated".to_string();

    let new_etag = client
        .update_contact(&created_href, &created_etag, &updated_contact)
        .await
        .expect("update_contact should succeed");

    // new_etag comes from the ETag header on the 204 response
    assert_eq!(new_etag, "\"etag-2\"", "update should return refreshed ETag");

    // DELETE
    client
        .delete_contact(&created_href, &new_etag, &contact.id)
        .await
        .expect("delete_contact should succeed");

    // --- Wire-format assertions ---
    let received = server
        .received_requests()
        .await
        .expect("received requests");

    let puts: Vec<_> = received
        .iter()
        .filter(|r| r.method.as_str() == "PUT")
        .collect();
    let deletes: Vec<_> = received
        .iter()
        .filter(|r| r.method.as_str() == "DELETE")
        .collect();

    assert_eq!(puts.len(), 2, "expected 2 PUTs (create + update), got {}", puts.len());
    assert_eq!(deletes.len(), 1, "expected 1 DELETE, got {}", deletes.len());

    // First PUT must carry If-None-Match: * (optimistic create guard)
    let first_put = &puts[0];
    let inm = first_put
        .headers
        .get("if-none-match")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert!(
        inm.contains('*'),
        "first PUT (create) should carry If-None-Match: *, got {inm:?}"
    );

    // Second PUT must carry If-Match with the previously captured ETag
    let second_put = &puts[1];
    let im = second_put
        .headers
        .get("if-match")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert!(
        im.contains("etag-1"),
        "second PUT (update) should carry If-Match: \"etag-1\", got {im:?}"
    );

    // DELETE must carry If-Match with the refreshed ETag
    let delete_req = &deletes[0];
    let delete_im = delete_req
        .headers
        .get("if-match")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert!(
        delete_im.contains("etag-2"),
        "DELETE should carry If-Match: \"etag-2\", got {delete_im:?}"
    );
}
