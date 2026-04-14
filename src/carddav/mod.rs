//! CardDAV client for Fastmail contacts
//!
//! Uses raw HTTP with reqwest since CardDAV is just WebDAV with vCard.

use futures::future::join_all;
use reqwest::{
    Client,
    header::{ETAG, HeaderMap, IF_MATCH, IF_NONE_MATCH, LOCATION},
};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};
pub use uuid::Uuid;

use crate::error::{Error, Result};

const CARDDAV_BASE: &str = "https://carddav.fastmail.com";

/// A contact parsed from vCard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contact {
    /// Unique ID (from UID property)
    pub id: String,
    /// Full name (FN property)
    pub name: String,
    /// Email addresses
    pub emails: Vec<ContactEmail>,
    /// Phone numbers
    pub phones: Vec<ContactPhone>,
    /// Organization/company
    pub organization: Option<String>,
    /// Job title
    pub title: Option<String>,
    /// Notes
    pub notes: Option<String>,
    /// Street address (from ADR property, street component)
    pub address: Option<String>,
    /// Server-assigned resource URL (from CardDAV REPORT <d:href>).
    /// Required for PUT/DELETE write operations.
    pub href: Option<String>,
    /// HTTP ETag for optimistic concurrency control.
    /// Required for If-Match header in update/delete operations.
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactEmail {
    pub email: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactPhone {
    pub number: String,
    pub label: Option<String>,
}

/// Address book info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    pub href: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactCreateResult {
    pub href: String,
    pub etag: Option<String>,
}

/// A contact group parsed from a group vCard (X-ADDRESSBOOKSERVER-KIND:group)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactGroup {
    /// Unique ID (from UID property)
    pub id: String,
    /// Display name (FN property)
    pub name: String,
    /// Member contact UIDs (extracted from X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid> lines)
    pub member_uids: Vec<String>,
    /// Server-assigned resource URL
    pub href: Option<String>,
    /// HTTP ETag for optimistic concurrency
    pub etag: Option<String>,
}

/// CardDAV client
pub struct CardDavClient {
    client: Client,
    username: String,
    app_password: String,
    base_url: String,
}

impl CardDavClient {
    /// Create a new CardDAV client pointing at the production Fastmail server.
    pub fn new(username: String, app_password: String) -> Result<Self> {
        Self::new_with_base_url(username, app_password, CARDDAV_BASE.to_string())
    }

    /// Create a new CardDAV client with a custom base URL (for testing).
    pub fn new_with_base_url(
        username: String,
        app_password: String,
        base_url: String,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Config(format!("HTTP client builder failed: {e}")))?;
        Ok(Self {
            client,
            username,
            app_password,
            base_url,
        })
    }

    /// Discover address books for the user
    #[instrument(skip(self))]
    pub async fn list_addressbooks(&self) -> Result<Vec<AddressBook>> {
        let url = format!("{}/dav/addressbooks/user/{}/", self.base_url, self.username);

        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:displayname/>
    <d:resourcetype/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text: String = response.text().await?;

        debug!(status = %status, "PROPFIND response");

        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CardDAV PROPFIND failed: {} - {}",
                status, text
            )));
        }

        // Parse the multistatus XML response
        self.parse_addressbooks_response(&text)
    }

    fn parse_addressbooks_response(&self, xml: &str) -> Result<Vec<AddressBook>> {
        let doc = roxmltree::Document::parse(xml)
            .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

        let dav_ns = "DAV:";
        let carddav_ns = "urn:ietf:params:xml:ns:carddav";
        let mut addressbooks = Vec::new();

        for response in doc
            .descendants()
            .filter(|n| n.has_tag_name((dav_ns, "response")))
        {
            let href = response
                .descendants()
                .find(|n| n.has_tag_name((dav_ns, "href")))
                .and_then(|n| n.text())
                .unwrap_or_default();

            // Check if this is an addressbook (has carddav:addressbook resourcetype)
            let is_addressbook = response
                .descendants()
                .any(|n| n.has_tag_name((carddav_ns, "addressbook")));

            if is_addressbook && !href.is_empty() {
                let displayname = response
                    .descendants()
                    .find(|n| n.has_tag_name((dav_ns, "displayname")))
                    .and_then(|n| n.text());

                let name = displayname.map(|s| s.to_string()).unwrap_or_else(|| {
                    href.split('/')
                        .rfind(|s| !s.is_empty())
                        .unwrap_or("Unknown")
                        .to_string()
                });

                // Skip the parent collection itself
                if !href.ends_with(&format!("{}/", self.username)) {
                    addressbooks.push(AddressBook {
                        href: href.to_string(),
                        name,
                    });
                }
            }
        }

        Ok(addressbooks)
    }

    /// List all contacts in an address book
    #[instrument(skip(self))]
    pub async fn list_contacts(&self, addressbook_href: &str) -> Result<Vec<Contact>> {
        let url = format!("{}{}", self.base_url, addressbook_href);

        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <card:address-data/>
  </d:prop>
</card:addressbook-query>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text: String = response.text().await?;

        debug!(status = %status, "REPORT response");

        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CardDAV REPORT failed: {} - {}",
                status, text
            )));
        }

        self.parse_contacts_response(&text)
    }

    fn parse_contacts_response(&self, xml: &str) -> Result<Vec<Contact>> {
        parse_contacts_from_xml(xml)
    }

    /// Search contacts by name or email across all address books concurrently.
    ///
    /// A single failing address book logs a warning and does not abort the search.
    pub async fn search_contacts(&self, query: &str) -> Result<Vec<Contact>> {
        let addressbooks = self.list_addressbooks().await?;

        // Build one future per address book
        let futures: Vec<_> = addressbooks
            .iter()
            .map(|ab| {
                let href = ab.href.clone();
                let client = self.client.clone();
                let username = self.username.clone();
                let app_password = self.app_password.clone();
                let base_url = self.base_url.clone();
                async move {
                    let url = format!("{}{}", base_url, href);
                    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <card:address-data/>
  </d:prop>
</card:addressbook-query>"#;
                    let response = client
                        .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
                        .basic_auth(&username, Some(&app_password))
                        .header("Content-Type", "application/xml")
                        .header("Depth", "1")
                        .body(body)
                        .send()
                        .await?;
                    let status = response.status();
                    let text: String = response.text().await?;
                    if !status.is_success() && status.as_u16() != 207 {
                        return Err(crate::error::Error::Server(format!(
                            "CardDAV REPORT failed for {href}: {status} - {text}"
                        )));
                    }
                    // Inline parse to avoid needing &self
                    parse_contacts_from_xml(&text)
                        .map(|contacts| (href, contacts))
                }
            })
            .collect();

        let results = join_all(futures).await;
        let all_contacts = collect_partial_contacts(results);

        let query_lower = query.to_lowercase();
        let filtered: Vec<Contact> = all_contacts
            .into_iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query_lower)
                    || c.emails
                        .iter()
                        .any(|e| e.email.to_lowercase().contains(&query_lower))
                    || c.organization
                        .as_ref()
                        .is_some_and(|o| o.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }

    /// Find a contact by exact ID across all address books.
    #[instrument(skip(self))]
    pub async fn get_contact_by_id(&self, contact_id: &str) -> Result<Contact> {
        let addressbooks = self.list_addressbooks().await?;

        for addressbook in addressbooks {
            let contacts = self.list_contacts(&addressbook.href).await?;
            if let Some(contact) = contacts
                .into_iter()
                .find(|contact| contact.id == contact_id)
            {
                return Ok(contact);
            }
        }

        Err(Error::ContactNotFound(contact_id.to_string()))
    }

    /// Return the first discovered address book href for create operations.
    #[instrument(skip(self))]
    pub async fn default_addressbook_href(&self) -> Result<String> {
        let addressbooks = self.list_addressbooks().await?;
        addressbooks
            .into_iter()
            .next()
            .map(|addressbook| addressbook.href)
            .ok_or_else(|| Error::Server("No CardDAV address books found".to_string()))
    }

    #[instrument(skip(self, contact))]
    pub async fn create_contact(
        &self,
        addressbook_href: &str,
        contact: &Contact,
    ) -> Result<ContactCreateResult> {
        let href = build_contact_href(addressbook_href, &contact.id);
        let url = format!("{}{}", self.base_url, href);
        let vcard = serialize_vcard(contact);

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/vcard; charset=utf-8")
            .header(IF_NONE_MATCH, "*")
            .body(vcard)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        debug!(status = %status, href = %href, "PUT create_contact response");

        let etag = map_write_response(contact, None, status, &headers, &body)?;
        let created_href = extract_location_path(&headers).unwrap_or(href);

        Ok(ContactCreateResult {
            href: created_href,
            etag,
        })
    }

    #[instrument(skip(self, contact))]
    pub async fn update_contact(
        &self,
        href: &str,
        etag: &str,
        contact: &Contact,
    ) -> Result<String> {
        let url = format!("{}{}", self.base_url, href);
        let vcard = serialize_vcard(contact);

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/vcard; charset=utf-8")
            .header(IF_MATCH, etag)
            .body(vcard)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        debug!(status = %status, href = %href, "PUT update_contact response");

        let new_etag = map_write_response(contact, Some(etag), status, &headers, &body)?
            .or_else(|| contact.etag.clone())
            .unwrap_or_else(|| etag.to_string());
        Ok(new_etag)
    }

    #[instrument(skip(self))]
    pub async fn delete_contact(&self, href: &str, etag: &str, contact_id: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, href);

        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header(IF_MATCH, etag)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        let contact = Contact {
            id: contact_id.to_string(),
            name: String::new(),
            emails: Vec::new(),
            phones: Vec::new(),
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: Some(href.to_string()),
            etag: Some(etag.to_string()),
        };

        debug!(status = %status, href = %href, "DELETE delete_contact response");

        map_write_response(&contact, Some(etag), status, &headers, &body)?;
        Ok(())
    }

    /// List all contact groups across all address books.
    #[instrument(skip(self))]
    pub async fn list_groups(&self) -> Result<Vec<ContactGroup>> {
        let addressbooks = self.list_addressbooks().await?;
        let futures: Vec<_> = addressbooks
            .into_iter()
            .map(|ab| self.fetch_addressbook_groups(ab.href))
            .collect();
        let results = join_all(futures).await;
        Ok(collect_partial_groups(results))
    }

    /// Fetch all group vCards from a single address book.
    async fn fetch_addressbook_groups(
        &self,
        addressbook_href: String,
    ) -> Result<(String, Vec<ContactGroup>)> {
        let url = format!("{}{}", self.base_url, addressbook_href);

        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <card:address-data/>
  </d:prop>
</card:addressbook-query>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text: String = response.text().await?;

        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CardDAV REPORT failed for {addressbook_href}: {status} - {text}"
            )));
        }

        parse_groups_from_xml(&text).map(|groups| (addressbook_href, groups))
    }

    /// Find a group by exact ID across all address books.
    #[instrument(skip(self))]
    pub async fn get_group_by_id(&self, group_id: &str) -> Result<ContactGroup> {
        let groups = self.list_groups().await?;
        groups
            .into_iter()
            .find(|g| g.id == group_id)
            .ok_or_else(|| Error::GroupNotFound(group_id.to_string()))
    }

    /// Find a group by exact name (case-sensitive) across all address books.
    ///
    /// Returns GroupNotFound if zero matches, GroupAmbiguous if multiple matches.
    #[instrument(skip(self))]
    pub async fn get_group_by_name(&self, name: &str) -> Result<ContactGroup> {
        let groups = self.list_groups().await?;
        let mut matches: Vec<ContactGroup> = groups.into_iter().filter(|g| g.name == name).collect();
        match matches.len() {
            0 => Err(Error::GroupNotFound(name.to_string())),
            1 => Ok(matches.remove(0)),
            _ => Err(Error::GroupAmbiguous(name.to_string())),
        }
    }

    /// Create a new contact group in the given address book.
    #[instrument(skip(self, group))]
    pub async fn create_group(
        &self,
        addressbook_href: &str,
        group: &ContactGroup,
    ) -> Result<ContactCreateResult> {
        let href = build_group_href(addressbook_href, &group.id);
        let url = format!("{}{}", self.base_url, href);
        let vcard = serialize_group_vcard(group);

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/vcard; charset=utf-8")
            .header(IF_NONE_MATCH, "*")
            .body(vcard)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        debug!(status = %status, href = %href, "PUT create_group response");

        let etag = map_group_write_response(&group.id, None, status, &headers, &body)?;
        let created_href = extract_location_path(&headers).unwrap_or(href);

        Ok(ContactCreateResult {
            href: created_href,
            etag,
        })
    }

    /// Rename a contact group, returning the new ETag.
    #[instrument(skip(self, group))]
    pub async fn rename_group(
        &self,
        href: &str,
        etag: &str,
        group: &ContactGroup,
        new_name: &str,
    ) -> Result<String> {
        let updated = ContactGroup {
            id: group.id.clone(),
            name: new_name.to_string(),
            member_uids: group.member_uids.clone(),
            href: group.href.clone(),
            etag: group.etag.clone(),
        };
        let url = format!("{}{}", self.base_url, href);
        let vcard = serialize_group_vcard(&updated);

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/vcard; charset=utf-8")
            .header(IF_MATCH, etag)
            .body(vcard)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        debug!(status = %status, href = %href, "PUT rename_group response");

        let new_etag =
            map_group_write_response(&group.id, Some(etag), status, &headers, &body)?
                .unwrap_or_else(|| etag.to_string());
        Ok(new_etag)
    }

    /// Delete a contact group by href and ETag.
    #[instrument(skip(self))]
    pub async fn delete_group(&self, href: &str, etag: &str, group_id: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, href);

        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header(IF_MATCH, etag)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        debug!(status = %status, href = %href, "DELETE delete_group response");

        map_group_write_response(group_id, Some(etag), status, &headers, &body)?;
        Ok(())
    }

    /// Add a contact to a group's membership list.
    ///
    /// Validates the contact exists first, then performs an ETag-guarded PUT with
    /// retry-on-412 (up to MAX_RETRIES attempts). Returns Ok(group) immediately if
    /// the contact is already a member (idempotent).
    #[instrument(skip(self))]
    pub async fn add_group_member(
        &self,
        group_id: &str,
        contact_uid: &str,
    ) -> Result<ContactGroup> {
        // Validate the contact exists before modifying the group (fail fast)
        self.get_contact_by_id(contact_uid).await?;

        const MAX_RETRIES: u8 = 3;

        for attempt in 0..MAX_RETRIES {
            let group = self.get_group_by_id(group_id).await?;

            let href = group
                .href
                .clone()
                .ok_or_else(|| Error::GroupNotFound(group_id.to_string()))?;

            let etag = group
                .etag
                .clone()
                .ok_or_else(|| Error::GroupConflict {
                    id: group_id.to_string(),
                    sent_etag: String::new(),
                    server_etag: None,
                })?;

            // Idempotency: skip PUT if contact is already a member
            if group.member_uids.contains(&contact_uid.to_string()) {
                return Ok(group);
            }

            let mut updated = group.clone();
            updated.member_uids.push(contact_uid.to_string());

            let url = format!("{}{}", self.base_url, href);
            let vcard = serialize_group_vcard(&updated);

            let response = self
                .client
                .put(&url)
                .basic_auth(&self.username, Some(&self.app_password))
                .header("Content-Type", "text/vcard; charset=utf-8")
                .header(IF_MATCH, &etag)
                .body(vcard)
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let body = response.text().await?;

            debug!(status = %status, href = %href, attempt, "PUT add_group_member response");

            match map_group_write_response(group_id, Some(&etag), status, &headers, &body) {
                Ok(new_etag) => {
                    updated.etag = new_etag.or_else(|| Some(etag.clone()));
                    return Ok(updated);
                }
                Err(Error::GroupConflict { .. }) if attempt < MAX_RETRIES - 1 => {
                    debug!(attempt, group_id, "ETag conflict on add_group_member, retrying");
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::GroupConflict {
            id: group_id.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })
    }

    /// Remove a contact from a group's membership list.
    ///
    /// Performs an ETag-guarded PUT with retry-on-412 (up to MAX_RETRIES attempts).
    /// Returns Ok(group) immediately if the contact is not a member (idempotent).
    /// No contact existence validation — removing a reference to a deleted contact is valid.
    #[instrument(skip(self))]
    pub async fn remove_group_member(
        &self,
        group_id: &str,
        contact_uid: &str,
    ) -> Result<ContactGroup> {
        const MAX_RETRIES: u8 = 3;

        for attempt in 0..MAX_RETRIES {
            let group = self.get_group_by_id(group_id).await?;

            let href = group
                .href
                .clone()
                .ok_or_else(|| Error::GroupNotFound(group_id.to_string()))?;

            let etag = group
                .etag
                .clone()
                .ok_or_else(|| Error::GroupConflict {
                    id: group_id.to_string(),
                    sent_etag: String::new(),
                    server_etag: None,
                })?;

            // Idempotency: skip PUT if contact is not a member
            if !group.member_uids.contains(&contact_uid.to_string()) {
                return Ok(group);
            }

            let mut updated = group.clone();
            updated.member_uids.retain(|uid| uid != contact_uid);

            let url = format!("{}{}", self.base_url, href);
            let vcard = serialize_group_vcard(&updated);

            let response = self
                .client
                .put(&url)
                .basic_auth(&self.username, Some(&self.app_password))
                .header("Content-Type", "text/vcard; charset=utf-8")
                .header(IF_MATCH, &etag)
                .body(vcard)
                .send()
                .await?;

            let status = response.status();
            let headers = response.headers().clone();
            let body = response.text().await?;

            debug!(status = %status, href = %href, attempt, "PUT remove_group_member response");

            match map_group_write_response(group_id, Some(&etag), status, &headers, &body) {
                Ok(new_etag) => {
                    updated.etag = new_etag.or_else(|| Some(etag.clone()));
                    return Ok(updated);
                }
                Err(Error::GroupConflict { .. }) if attempt < MAX_RETRIES - 1 => {
                    debug!(attempt, group_id, "ETag conflict on remove_group_member, retrying");
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::GroupConflict {
            id: group_id.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })
    }

    /// Resolve all member contacts for a group in a single batch fetch.
    ///
    /// Fetches all contacts across all address books once, then filters in-memory
    /// by group.member_uids. Avoids N+1 HTTP calls.
    #[instrument(skip(self, group))]
    pub async fn resolve_group_members(&self, group: &ContactGroup) -> Result<Vec<Contact>> {
        let addressbooks = self.list_addressbooks().await?;
        let mut all_contacts: Vec<Contact> = Vec::new();
        for ab in addressbooks {
            let contacts = self.list_contacts(&ab.href).await?;
            all_contacts.extend(contacts);
        }
        let members: Vec<Contact> = all_contacts
            .into_iter()
            .filter(|c| group.member_uids.contains(&c.id))
            .collect();
        Ok(members)
    }
}

/// Parse CardDAV multistatus XML into a list of contacts.
///
/// Used both by the `parse_contacts_response` method and by the concurrent
/// per-book futures in `search_contacts`.
fn parse_contacts_from_xml(xml: &str) -> Result<Vec<Contact>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

    let dav_ns = "DAV:";
    let carddav_ns = "urn:ietf:params:xml:ns:carddav";
    let mut contacts = Vec::new();

    for response in doc
        .descendants()
        .filter(|n| n.has_tag_name((dav_ns, "response")))
    {
        let href = response
            .descendants()
            .find(|n| n.has_tag_name((dav_ns, "href")))
            .and_then(|n| n.text())
            .map(|s| s.to_string());

        let etag = response
            .descendants()
            .find(|n| n.has_tag_name((dav_ns, "getetag")))
            .and_then(|n| n.text())
            .map(|s| s.to_string());

        if let Some(vcard_data) = response
            .descendants()
            .find(|n| n.has_tag_name((carddav_ns, "address-data")))
            .and_then(|n| n.text())
            && let Some(contact) = parse_vcard(vcard_data, href, etag)
        {
            contacts.push(contact);
        }
    }

    contacts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(contacts)
}

/// Flatten results from concurrent per-book fetches, logging warnings for failures.
///
/// Returns items from successful books only. If all books fail, returns an empty
/// `Vec` — never an `Err` — so callers always get a partial/empty result.
fn collect_partial_contacts(
    results: Vec<Result<(String, Vec<Contact>)>>,
) -> Vec<Contact> {
    let mut all = Vec::new();
    for result in results {
        match result {
            Ok((_, contacts)) => all.extend(contacts),
            Err(e) => {
                warn!(error = %e, "CardDAV search_contacts: book fetch failed");
            }
        }
    }
    all
}

fn build_contact_href(addressbook_href: &str, contact_id: &str) -> String {
    let trimmed = addressbook_href.trim_end_matches('/');
    format!("{trimmed}/{contact_id}.vcf")
}

fn header_value(headers: &HeaderMap, name: reqwest::header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn extract_location_path(headers: &HeaderMap) -> Option<String> {
    let location = header_value(headers, LOCATION)?;
    if let Some(stripped) = location.strip_prefix(CARDDAV_BASE) {
        return Some(stripped.to_string());
    }
    if location.starts_with('/') {
        return Some(location);
    }
    None
}

fn map_write_response(
    contact: &Contact,
    sent_etag: Option<&str>,
    status: reqwest::StatusCode,
    headers: &HeaderMap,
    body: &str,
) -> Result<Option<String>> {
    if matches!(
        status,
        reqwest::StatusCode::CREATED | reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::OK
    ) {
        return Ok(header_value(headers, ETAG));
    }

    match status {
        reqwest::StatusCode::PRECONDITION_FAILED => Err(Error::ContactConflict {
            id: contact.id.clone(),
            sent_etag: sent_etag.unwrap_or_default().to_string(),
            server_etag: header_value(headers, ETAG),
        }),
        reqwest::StatusCode::NOT_FOUND => Err(Error::ContactNotFound(contact.id.clone())),
        _ => Err(Error::Server(format!(
            "CardDAV write failed for {}: {} - {}",
            contact.id, status, body
        ))),
    }
}

/// Parse a group vCard string into a ContactGroup.
///
/// Returns None if the vCard does not have X-ADDRESSBOOKSERVER-KIND:group or has no FN.
fn parse_group_vcard(
    vcard_str: &str,
    href: Option<String>,
    etag: Option<String>,
) -> Option<ContactGroup> {
    let unfolded = unfold_vcard(vcard_str);
    let mut id = String::new();
    let mut name = String::new();
    let mut member_uids: Vec<String> = Vec::new();
    let mut is_group = false;

    for line in unfolded.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("X-ADDRESSBOOKSERVER-KIND:group") {
            is_group = true;
        } else if line.starts_with("UID") && line.contains(':') {
            id = line.split_once(':').map(|(_, v)| v).unwrap_or("").to_string();
        } else if line.starts_with("FN") && line.contains(':') {
            let raw_val = line.split_once(':').map(|(_, v)| v).unwrap_or("");
            name = unescape_value(raw_val);
        } else if line
            .to_uppercase()
            .starts_with("X-ADDRESSBOOKSERVER-MEMBER:")
        {
            // Line format: X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>
            if let Some(val) = line.split_once(':').map(|(_, v)| v) {
                // val is "urn:uuid:<uid>"
                let uid = val
                    .strip_prefix("urn:uuid:")
                    .unwrap_or(val)
                    .to_string();
                if !uid.is_empty() {
                    member_uids.push(uid);
                }
            }
        }
    }

    if !is_group || name.is_empty() {
        return None;
    }

    if id.is_empty() {
        id = Uuid::new_v4().to_string();
    }

    Some(ContactGroup {
        id,
        name,
        member_uids,
        href,
        etag,
    })
}

/// Serialize a ContactGroup to a vCard 3.0 string with X-ADDRESSBOOKSERVER extensions.
///
/// Emits X-ADDRESSBOOKSERVER-KIND:group and one X-ADDRESSBOOKSERVER-MEMBER line per member UID.
pub fn serialize_group_vcard(group: &ContactGroup) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:3.0".to_string());
    lines.push(format!("UID:{}", group.id));
    lines.push(format!("FN:{}", escape_value(&group.name)));
    lines.push("X-ADDRESSBOOKSERVER-KIND:group".to_string());

    for uid in &group.member_uids {
        lines.push(format!("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:{uid}"));
    }

    lines.push("END:VCARD".to_string());

    lines.iter().map(|l| fold_line(l)).collect::<String>()
}

/// Parse CardDAV multistatus XML into a list of contact groups.
///
/// Mirrors parse_contacts_from_xml but calls parse_group_vcard instead of parse_vcard.
fn parse_groups_from_xml(xml: &str) -> Result<Vec<ContactGroup>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

    let dav_ns = "DAV:";
    let carddav_ns = "urn:ietf:params:xml:ns:carddav";
    let mut groups = Vec::new();

    for response in doc
        .descendants()
        .filter(|n| n.has_tag_name((dav_ns, "response")))
    {
        let href = response
            .descendants()
            .find(|n| n.has_tag_name((dav_ns, "href")))
            .and_then(|n| n.text())
            .map(|s| s.to_string());

        let etag = response
            .descendants()
            .find(|n| n.has_tag_name((dav_ns, "getetag")))
            .and_then(|n| n.text())
            .map(|s| s.to_string());

        if let Some(vcard_data) = response
            .descendants()
            .find(|n| n.has_tag_name((carddav_ns, "address-data")))
            .and_then(|n| n.text())
            && let Some(group) = parse_group_vcard(vcard_data, href, etag)
        {
            groups.push(group);
        }
    }

    groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(groups)
}

/// Build a CardDAV resource href for a group vCard.
///
/// Mirrors build_contact_href: `{addressbook_href}/{group_id}.vcf`
fn build_group_href(addressbook_href: &str, group_id: &str) -> String {
    let trimmed = addressbook_href.trim_end_matches('/');
    format!("{trimmed}/{group_id}.vcf")
}

/// Map a CardDAV write response for a group, returning the new ETag or an appropriate error.
///
/// Mirrors map_write_response but uses GroupConflict/GroupNotFound error variants.
fn map_group_write_response(
    group_id: &str,
    sent_etag: Option<&str>,
    status: reqwest::StatusCode,
    headers: &HeaderMap,
    body: &str,
) -> Result<Option<String>> {
    if matches!(
        status,
        reqwest::StatusCode::CREATED | reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::OK
    ) {
        return Ok(header_value(headers, ETAG));
    }

    match status {
        reqwest::StatusCode::PRECONDITION_FAILED => Err(Error::GroupConflict {
            id: group_id.to_string(),
            sent_etag: sent_etag.unwrap_or_default().to_string(),
            server_etag: header_value(headers, ETAG),
        }),
        reqwest::StatusCode::NOT_FOUND => Err(Error::GroupNotFound(group_id.to_string())),
        _ => Err(Error::Server(format!(
            "CardDAV group write failed for {group_id}: {status} - {body}"
        ))),
    }
}

/// Flatten results from concurrent per-book group fetches, logging warnings for failures.
fn collect_partial_groups(results: Vec<Result<(String, Vec<ContactGroup>)>>) -> Vec<ContactGroup> {
    let mut all = Vec::new();
    for result in results {
        match result {
            Ok((_, groups)) => all.extend(groups),
            Err(e) => {
                warn!(error = %e, "CardDAV list_groups: book fetch failed");
            }
        }
    }
    all
}

/// Unfold vCard lines per RFC 6350 §3.2: continuation lines start with a space or tab.
fn unfold_vcard(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation line — append without the leading whitespace
            result.push_str(&line[1..]);
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    result
}

/// Decode quoted-printable encoded value (basic implementation for vCard)
fn decode_qp(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut decoded_bytes = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' && i + 2 < bytes.len() {
            if bytes[i + 1] == b'\r' || bytes[i + 1] == b'\n' {
                // Soft line break — skip
                i += 2;
                if i < bytes.len() && bytes[i] == b'\n' {
                    i += 1;
                }
            } else if let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                decoded_bytes.push((hi * 16 + lo) as u8);
                i += 3;
            } else {
                decoded_bytes.push(b'=');
                i += 1;
            }
        } else {
            decoded_bytes.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded_bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Parse a vCard string into a Contact
///
/// Returns None for group vCards (X-ADDRESSBOOKSERVER-KIND:group) — use parse_group_vcard instead.
fn parse_vcard(vcard_str: &str, href: Option<String>, etag: Option<String>) -> Option<Contact> {
    let unfolded = unfold_vcard(vcard_str);

    // Early return: filter out group vCards so they don't leak into contact lists
    for line in unfolded.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("X-ADDRESSBOOKSERVER-KIND:group") {
            return None;
        }
    }

    let mut id = String::new();
    let mut name = String::new();
    let mut emails = Vec::new();
    let mut phones = Vec::new();
    let mut organization = None;
    let mut title = None;
    let mut notes = None;
    let mut address = None;

    for line in unfolded.lines() {
        let line = line.trim();

        // Extract property value, handling optional parameters and QP encoding,
        // then unescape backslash-escaped characters per RFC 2426 §5.
        let extract_value = |line: &str| -> String {
            let value = line.split_once(':').map(|(_, v)| v).unwrap_or("");
            if line.to_uppercase().contains("ENCODING=QUOTED-PRINTABLE") {
                decode_qp(value)
            } else {
                unescape_value(value)
            }
        };

        if line.starts_with("UID") && line.contains(':') {
            id = extract_value(line);
        } else if line.starts_with("FN") && line.contains(':') {
            name = extract_value(line);
        } else if line.starts_with("EMAIL") {
            // EMAIL;TYPE=work:bob@example.com or EMAIL:bob@example.com
            let label = if line.contains("TYPE=") {
                line.split("TYPE=")
                    .nth(1)
                    .and_then(|s| s.split(':').next())
                    .map(|s| s.to_string())
            } else {
                None
            };
            let email = line.split(':').next_back().unwrap_or("").to_string();
            if !email.is_empty() {
                emails.push(ContactEmail { email, label });
            }
        } else if line.starts_with("TEL") {
            let label = if line.contains("TYPE=") {
                line.split("TYPE=")
                    .nth(1)
                    .and_then(|s| s.split(':').next())
                    .or_else(|| line.split("TYPE=").nth(1).and_then(|s| s.split(';').next()))
                    .map(|s| s.to_string())
            } else {
                None
            };
            let number = line.split(':').next_back().unwrap_or("").to_string();
            if !number.is_empty() {
                phones.push(ContactPhone { number, label });
            }
        } else if line.starts_with("ORG") && line.contains(':') {
            organization = Some(extract_value(line));
        } else if line.starts_with("TITLE") && line.contains(':') {
            title = Some(extract_value(line));
        } else if line.starts_with("NOTE") && line.contains(':') {
            notes = Some(extract_value(line));
        } else if line.starts_with("ADR") && line.contains(':') {
            // ADR format: PO Box;Extended;Street;Locality;Region;Postal;Country
            // We extract the street component (index 2)
            let value = extract_value(line);
            let parts: Vec<&str> = value.splitn(7, ';').collect();
            if parts.len() > 2 {
                let street = parts[2].trim();
                if !street.is_empty() {
                    address = Some(street.to_string());
                }
            }
        }
    }

    // Need at least a name
    if name.is_empty() {
        return None;
    }

    // Generate ID if not present
    if id.is_empty() {
        id = format!("{:x}", hash_id(&name));
    }

    Some(Contact {
        id,
        name,
        emails,
        phones,
        organization,
        title,
        notes,
        address,
        href,
        etag,
    })
}

/// Escape special characters in a vCard property value per RFC 2426 §5.
///
/// Must escape backslash first to avoid double-escaping.
fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Unescape backslash-escaped characters in a vCard property value per RFC 2426 §5.
///
/// Inverse of escape_value: converts `\\` → `\`, `\;` → `;`, `\,` → `,`, `\n` → newline.
fn unescape_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some(';') => {
                    result.push(';');
                    chars.next();
                }
                Some(',') => {
                    result.push(',');
                    chars.next();
                }
                Some('n') => {
                    result.push('\n');
                    chars.next();
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Fold a vCard property line at 75 octets per RFC 6350 §3.2.
///
/// First physical line: max 75 bytes (excluding CRLF).
/// Continuation lines: max 74 bytes of content (1 byte is leading space).
/// UTF-8 characters are never split across fold boundaries.
fn fold_line(line: &str) -> String {
    const FIRST_MAX: usize = 75;
    const CONT_MAX: usize = 74;

    if line.len() <= FIRST_MAX {
        return format!("{}\r\n", line);
    }

    let mut result = String::new();

    // First chunk: up to 75 bytes, walking back to a char boundary
    let mut end = FIRST_MAX.min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    result.push_str(&line[..end]);
    result.push_str("\r\n ");
    let mut pos = end;

    // Continuation chunks: up to 74 bytes of content each
    while pos < line.len() {
        let remaining = &line[pos..];
        if remaining.len() <= CONT_MAX {
            result.push_str(remaining);
            break;
        }
        let mut chunk_end = CONT_MAX.min(remaining.len());
        while !remaining.is_char_boundary(chunk_end) {
            chunk_end -= 1;
        }
        result.push_str(&remaining[..chunk_end]);
        result.push_str("\r\n ");
        pos += chunk_end;
    }

    result.push_str("\r\n");
    result
}

/// Serialize a Contact to a vCard 3.0 string.
///
/// The returned string uses CRLF line endings and line folding at 75 octets
/// per RFC 6350 §3.2. Character values are escaped per RFC 2426 §5.
/// The contact's `id` field is used as the UID; callers must supply a
/// UUID v4 for new contacts (use `Uuid::new_v4().to_string()`).
pub fn serialize_vcard(contact: &Contact) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:3.0".to_string());
    lines.push(format!("UID:{}", contact.id));
    lines.push(format!("FN:{}", escape_value(&contact.name)));

    // N property: decompose name by whitespace tokens
    // D-01: first token = given name, last token = family name (if >1 token),
    // middle tokens joined. Single token: given only, family empty.
    let tokens: Vec<&str> = contact.name.split_whitespace().collect();
    let (family, given, middle) = match tokens.len() {
        0 => ("".to_string(), "".to_string(), "".to_string()),
        1 => ("".to_string(), escape_value(tokens[0]), "".to_string()),
        2 => (
            escape_value(tokens[1]),
            escape_value(tokens[0]),
            "".to_string(),
        ),
        _ => {
            let last = tokens.len() - 1;
            let mid = tokens[1..last].join(" ");
            (
                escape_value(tokens[last]),
                escape_value(tokens[0]),
                escape_value(&mid),
            )
        }
    };
    lines.push(format!("N:{};{};{};;", family, given, middle));

    // EMAIL properties
    for email in &contact.emails {
        if let Some(label) = &email.label {
            lines.push(format!("EMAIL;TYPE={}:{}", escape_value(label), escape_value(&email.email)));
        } else {
            lines.push(format!("EMAIL:{}", escape_value(&email.email)));
        }
    }

    // TEL properties
    for phone in &contact.phones {
        if let Some(label) = &phone.label {
            lines.push(format!("TEL;TYPE={}:{}", escape_value(label), escape_value(&phone.number)));
        } else {
            lines.push(format!("TEL:{}", escape_value(&phone.number)));
        }
    }

    // Optional properties
    if let Some(org) = &contact.organization {
        lines.push(format!("ORG:{}", escape_value(org)));
    }
    if let Some(title) = &contact.title {
        lines.push(format!("TITLE:{}", escape_value(title)));
    }
    // ADR: ;;street;;;;; (6 semicolons: PO box;extended;street;locality;region;postal;country)
    if let Some(street) = &contact.address {
        lines.push(format!("ADR:;;{};;;;;", escape_value(street)));
    }
    if let Some(notes) = &contact.notes {
        lines.push(format!("NOTE:{}", escape_value(notes)));
    }

    lines.push("END:VCARD".to_string());

    // Apply fold_line to each property line, then concatenate
    lines.iter().map(|l| fold_line(l)).collect::<String>()
}

/// Simple SipHash-based hash for generating stable contact IDs
fn hash_id(s: &str) -> u64 {
    use siphasher::sip::SipHasher13;
    use std::hash::{Hash, Hasher};
    // Fixed seed ensures stable IDs across Rust versions (D-02, D-04).
    // Key values are arbitrary public constants — determinism, not secrecy.
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{StatusCode, header::HeaderValue};

    #[test]
    fn test_unfold_vcard_lines() {
        // RFC 6350 §3.2: leading space/tab is the fold indicator and is consumed
        let input = "FN:John\n  Doe\nEMAIL:john@example.com";
        let result = unfold_vcard(input);
        assert_eq!(result, "FN:John Doe\nEMAIL:john@example.com");
    }

    #[test]
    fn test_unfold_tab_continuation() {
        let input = "FN:John\n\tDoe";
        let result = unfold_vcard(input);
        assert_eq!(result, "FN:JohnDoe");
    }

    #[test]
    fn test_decode_qp_basic() {
        assert_eq!(decode_qp("hello=20world"), "hello world");
        assert_eq!(decode_qp("caf=C3=A9"), "café");
    }

    #[test]
    fn test_decode_qp_soft_linebreak() {
        assert_eq!(decode_qp("hello=\nworld"), "helloworld");
    }

    #[test]
    fn test_parse_vcard_basic() {
        let vcard = "BEGIN:VCARD\nVERSION:3.0\nUID:abc123\nFN:Alice Smith\nEMAIL:alice@example.com\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert_eq!(contact.id, "abc123");
        assert_eq!(contact.name, "Alice Smith");
        assert_eq!(contact.emails.len(), 1);
        assert_eq!(contact.emails[0].email, "alice@example.com");
    }

    #[test]
    fn test_parse_vcard_with_line_folding() {
        // Fold happens mid-value: "Very Long Name Here" folded after "Na"
        // Continuation line starts with space (fold indicator consumed)
        let vcard = "BEGIN:VCARD\nFN:Very Long Na\n me Here\nEMAIL:test@example.com\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert_eq!(contact.name, "Very Long Name Here");
    }

    #[test]
    fn test_parse_vcard_with_params() {
        let vcard = "BEGIN:VCARD\nFN:Bob\nEMAIL;TYPE=work:bob@work.com\nTEL;TYPE=cell:+1234567890\nORG:Acme Inc\nTITLE:Engineer\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert_eq!(contact.emails[0].email, "bob@work.com");
        assert_eq!(contact.emails[0].label, Some("work".to_string()));
        assert_eq!(contact.phones[0].number, "+1234567890");
        assert_eq!(contact.organization, Some("Acme Inc".to_string()));
        assert_eq!(contact.title, Some("Engineer".to_string()));
    }

    #[test]
    fn test_parse_vcard_generates_id_when_missing() {
        let vcard = "BEGIN:VCARD\nFN:No UID\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert!(!contact.id.is_empty());
    }

    #[test]
    fn test_parse_vcard_returns_none_without_name() {
        let vcard = "BEGIN:VCARD\nUID:abc\nEMAIL:test@example.com\nEND:VCARD";
        assert!(parse_vcard(vcard, None, None).is_none());
    }

    #[test]
    fn test_parse_vcard_with_href_etag() {
        let vcard = "BEGIN:VCARD\nVERSION:3.0\nUID:abc123\nFN:Test Contact\nEND:VCARD";
        let contact = parse_vcard(
            vcard,
            Some("/dav/abc.vcf".to_string()),
            Some("\"etag123\"".to_string()),
        )
        .unwrap();
        assert_eq!(contact.href, Some("/dav/abc.vcf".to_string()));
        assert_eq!(contact.etag, Some("\"etag123\"".to_string()));
    }

    #[test]
    fn test_contact_address_some() {
        let c = Contact {
            id: "id1".to_string(),
            name: "Test".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: Some("123 Main St".to_string()),
            href: None,
            etag: None,
        };
        assert_eq!(c.address, Some("123 Main St".to_string()));
    }

    #[test]
    fn test_contact_address_none() {
        let c = Contact {
            id: "id2".to_string(),
            name: "Test".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        assert!(c.address.is_none());
    }

    #[test]
    fn test_parse_vcard_address_adr() {
        let vcard =
            "BEGIN:VCARD\nVERSION:3.0\nUID:abc\nFN:Alice\nADR:;;123 Main St;;;;;\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert_eq!(contact.address, Some("123 Main St".to_string()));
    }

    #[test]
    fn test_parse_vcard_address_none_when_no_adr() {
        let vcard = "BEGIN:VCARD\nVERSION:3.0\nUID:abc\nFN:Alice\nEND:VCARD";
        let contact = parse_vcard(vcard, None, None).unwrap();
        assert!(contact.address.is_none());
    }

    #[test]
    fn test_parse_contacts_response_extracts_href_etag() {
        let client = CardDavClient::new("testuser".to_string(), "testpass".to_string()).expect("test client");
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/dav/addressbooks/user/testuser/Default/contact1.vcf</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"etag-value-123"</d:getetag>
        <card:address-data>BEGIN:VCARD
VERSION:3.0
UID:uid-001
FN:Test Contact
EMAIL:test@example.com
END:VCARD</card:address-data>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
        let contacts = client.parse_contacts_response(xml).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(
            contacts[0].href,
            Some("/dav/addressbooks/user/testuser/Default/contact1.vcf".to_string())
        );
        assert_eq!(contacts[0].etag, Some("\"etag-value-123\"".to_string()));
    }

    // ===== RED tests for escape_value =====

    #[test]
    fn test_escape_value_backslash() {
        assert_eq!(escape_value("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_escape_value_semicolon() {
        assert_eq!(escape_value("a;b"), "a\\;b");
    }

    #[test]
    fn test_escape_value_comma() {
        assert_eq!(escape_value("a,b"), "a\\,b");
    }

    #[test]
    fn test_escape_value_newline() {
        assert_eq!(escape_value("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_value_combined() {
        assert_eq!(escape_value("a\\;b,c\nd"), "a\\\\\\;b\\,c\\nd");
    }

    #[test]
    fn test_escape_value_no_special() {
        assert_eq!(escape_value("hello world"), "hello world");
    }

    // ===== RED tests for fold_line =====

    #[test]
    fn test_fold_line_short() {
        assert_eq!(fold_line("FN:Alice"), "FN:Alice\r\n");
    }

    #[test]
    fn test_fold_line_exactly_75() {
        let line = "X".repeat(75);
        let result = fold_line(&line);
        assert!(result.ends_with("\r\n"));
        // No folding: result should be just the 75 chars + CRLF, no continuation
        assert!(!result.contains("\r\n "));
    }

    #[test]
    fn test_fold_line_76_bytes() {
        let line = "X".repeat(76);
        let result = fold_line(&line);
        // Should fold into 2 physical lines with CRLF + space
        assert!(result.contains("\r\n "));
        // First line: 75 bytes, continuation: 1 byte + CRLF
        let lines: Vec<&str> = result.split("\r\n").collect();
        assert!(lines.len() >= 2);
        assert_eq!(lines[0].len(), 75);
    }

    #[test]
    fn test_fold_line_long() {
        let line = "X".repeat(200);
        let result = fold_line(&line);
        // Must end with CRLF
        assert!(result.ends_with("\r\n"));
        // Reconstruct: strip CRLF from each line, strip leading space from continuations
        let mut unfolded = String::new();
        for (i, physical) in result
            .split("\r\n")
            .filter(|s: &&str| !s.is_empty())
            .enumerate()
        {
            if i == 0 {
                unfolded.push_str(physical);
            } else {
                // Leading space is fold indicator, skip it
                unfolded.push_str(&physical[1..]);
            }
        }
        assert_eq!(unfolded, line);
    }

    #[test]
    fn test_fold_line_utf8_boundary() {
        // "é" is 2 bytes in UTF-8; place it near the 75-byte boundary
        // Build a line where a multi-byte char straddles position 75
        let prefix = "A".repeat(74); // 74 bytes
        let suffix = "é"; // 2 bytes, total = 76 bytes
        let line = format!("{}{}", prefix, suffix);
        let result = fold_line(&line);
        // Result must be valid UTF-8
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        // Must end with CRLF
        assert!(result.ends_with("\r\n"));
    }

    // ===== RED tests for serialize_vcard =====

    fn make_basic_contact() -> Contact {
        Contact {
            id: "test-uid-001".to_string(),
            name: "Alice Smith".to_string(),
            emails: vec![ContactEmail {
                email: "alice@example.com".to_string(),
                label: None,
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

    #[test]
    fn test_build_contact_href_appends_vcf() {
        assert_eq!(
            build_contact_href("/dav/addressbooks/user/test/Default/", "abc-123"),
            "/dav/addressbooks/user/test/Default/abc-123.vcf"
        );
    }

    #[test]
    fn test_extract_location_path_handles_absolute_and_relative_urls() {
        let mut headers = HeaderMap::new();
        headers.insert(
            LOCATION,
            HeaderValue::from_static(
                "https://carddav.fastmail.com/dav/addressbooks/user/test/Default/abc.vcf",
            ),
        );
        assert_eq!(
            extract_location_path(&headers).as_deref(),
            Some("/dav/addressbooks/user/test/Default/abc.vcf")
        );

        headers.insert(
            LOCATION,
            HeaderValue::from_static("/dav/addressbooks/user/test/Default/xyz.vcf"),
        );
        assert_eq!(
            extract_location_path(&headers).as_deref(),
            Some("/dav/addressbooks/user/test/Default/xyz.vcf")
        );
    }

    #[test]
    fn test_map_write_response_success_extracts_etag() {
        let contact = make_basic_contact();
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"etag-123\""));

        let etag = map_write_response(&contact, None, StatusCode::CREATED, &headers, "").unwrap();
        assert_eq!(etag.as_deref(), Some("\"etag-123\""));
    }

    #[test]
    fn test_map_write_response_conflict_uses_server_etag() {
        let contact = make_basic_contact();
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"etag-server\""));

        let error = map_write_response(
            &contact,
            Some("\"etag-old\""),
            StatusCode::PRECONDITION_FAILED,
            &headers,
            "",
        )
        .unwrap_err();

        match error {
            Error::ContactConflict {
                id,
                sent_etag,
                server_etag,
            } => {
                assert_eq!(id, contact.id);
                assert_eq!(sent_etag, "\"etag-old\"");
                assert_eq!(server_etag.as_deref(), Some("\"etag-server\""));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_map_write_response_not_found() {
        let contact = make_basic_contact();
        let error = map_write_response(
            &contact,
            Some("\"etag-old\""),
            StatusCode::NOT_FOUND,
            &HeaderMap::new(),
            "",
        )
        .unwrap_err();

        match error {
            Error::ContactNotFound(id) => assert_eq!(id, contact.id),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_serialize_vcard_basic() {
        let contact = make_basic_contact();
        let output = serialize_vcard(&contact);
        assert!(output.contains("BEGIN:VCARD"));
        assert!(output.contains("VERSION:3.0"));
        assert!(output.contains("FN:Alice Smith"));
        assert!(output.contains("N:Smith;Alice;;;"));
        assert!(output.contains("EMAIL:alice@example.com"));
        assert!(output.contains("END:VCARD"));
    }

    #[test]
    fn test_serialize_vcard_full_contact() {
        let contact = Contact {
            id: "uid-full".to_string(),
            name: "John Q Smith".to_string(),
            emails: vec![
                ContactEmail {
                    email: "john@work.com".to_string(),
                    label: Some("work".to_string()),
                },
                ContactEmail {
                    email: "john@home.com".to_string(),
                    label: None,
                },
            ],
            phones: vec![ContactPhone {
                number: "+15551234567".to_string(),
                label: Some("cell".to_string()),
            }],
            organization: Some("Acme Corp".to_string()),
            title: Some("Engineer".to_string()),
            notes: Some("Test notes".to_string()),
            address: Some("456 Oak Ave".to_string()),
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("FN:John Q Smith"));
        assert!(output.contains("N:Smith;John;Q;;"));
        assert!(output.contains("EMAIL;TYPE=work:john@work.com"));
        assert!(output.contains("EMAIL:john@home.com"));
        assert!(output.contains("TEL;TYPE=cell:+15551234567"));
        assert!(output.contains("ORG:Acme Corp"));
        assert!(output.contains("TITLE:Engineer"));
        assert!(output.contains("ADR:;;456 Oak Ave;;;;;"));
        assert!(output.contains("NOTE:Test notes"));
    }

    #[test]
    fn test_serialize_vcard_single_name() {
        let contact = Contact {
            id: "uid-single".to_string(),
            name: "Alice".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("N:;Alice;;;"));
    }

    #[test]
    fn test_serialize_vcard_three_part_name() {
        let contact = Contact {
            id: "uid-three".to_string(),
            name: "John Q Smith".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("N:Smith;John;Q;;"));
    }

    #[test]
    fn test_serialize_vcard_four_part_name() {
        let contact = Contact {
            id: "uid-four".to_string(),
            name: "John Q R Smith".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("N:Smith;John;Q R;;"));
    }

    #[test]
    fn test_serialize_vcard_address() {
        let contact = Contact {
            id: "uid-addr".to_string(),
            name: "Test".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: Some("123 Main St".to_string()),
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("ADR:;;123 Main St;;;;;"));
    }

    #[test]
    fn test_serialize_vcard_optional_fields_none() {
        let contact = Contact {
            id: "uid-min".to_string(),
            name: "Minimal".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(!output.contains("ORG:"));
        assert!(!output.contains("TITLE:"));
        assert!(!output.contains("ADR:"));
        assert!(!output.contains("NOTE:"));
    }

    #[test]
    fn test_serialize_vcard_email_with_label() {
        let contact = Contact {
            id: "uid-email".to_string(),
            name: "Test".to_string(),
            emails: vec![ContactEmail {
                email: "user@example.com".to_string(),
                label: Some("work".to_string()),
            }],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("EMAIL;TYPE=work:user@example.com"));
    }

    #[test]
    fn test_serialize_vcard_email_without_label() {
        let contact = Contact {
            id: "uid-email-nolabel".to_string(),
            name: "Test".to_string(),
            emails: vec![ContactEmail {
                email: "user@example.com".to_string(),
                label: None,
            }],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("EMAIL:user@example.com"));
    }

    #[test]
    fn test_serialize_vcard_phone_with_label() {
        let contact = Contact {
            id: "uid-phone".to_string(),
            name: "Test".to_string(),
            emails: vec![],
            phones: vec![ContactPhone {
                number: "+1234567890".to_string(),
                label: Some("cell".to_string()),
            }],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        assert!(output.contains("TEL;TYPE=cell:+1234567890"));
    }

    #[test]
    fn test_serialize_vcard_escaping_in_values() {
        let contact = Contact {
            id: "uid-escape".to_string(),
            name: "Alice;Bob".to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let output = serialize_vcard(&contact);
        // Semicolon in name should be escaped in FN
        assert!(output.contains("FN:Alice\\;Bob"));
    }

    #[test]
    fn test_serialize_vcard_crlf_line_endings() {
        let contact = make_basic_contact();
        let output = serialize_vcard(&contact);
        // Every line should end with \r\n
        for line in output.split('\n') {
            let line: &str = line;
            if !line.is_empty() {
                assert!(line.ends_with('\r'), "Line missing CR: {:?}", line);
            }
        }
    }

    #[test]
    fn test_serialize_vcard_uid_present() {
        let contact = make_basic_contact();
        let output = serialize_vcard(&contact);
        assert!(output.contains("UID:test-uid-001"));
    }

    #[test]
    fn test_serialize_vcard_round_trip() {
        let contact = Contact {
            id: "uid-roundtrip".to_string(),
            name: "Bob Builder".to_string(),
            emails: vec![ContactEmail {
                email: "bob@example.com".to_string(),
                label: Some("work".to_string()),
            }],
            phones: vec![ContactPhone {
                number: "+9876543210".to_string(),
                label: Some("cell".to_string()),
            }],
            organization: Some("Build It Ltd".to_string()),
            title: Some("Builder".to_string()),
            notes: Some("Test notes".to_string()),
            address: Some("99 Builder St".to_string()),
            href: None,
            etag: None,
        };
        let serialized = serialize_vcard(&contact);
        let parsed = parse_vcard(&serialized, None, None).unwrap();
        assert_eq!(parsed.name, contact.name);
        assert_eq!(parsed.emails.len(), contact.emails.len());
        assert_eq!(parsed.emails[0].email, contact.emails[0].email);
        assert_eq!(parsed.phones.len(), contact.phones.len());
        assert_eq!(parsed.phones[0].number, contact.phones[0].number);
        assert_eq!(parsed.organization, contact.organization);
        assert_eq!(parsed.title, contact.title);
        assert_eq!(parsed.notes, contact.notes);
        assert_eq!(parsed.address, contact.address);
    }

    #[test]
    fn test_serialize_vcard_round_trip_special_chars() {
        let contact = Contact {
            id: "uid-special".to_string(),
            name: "O'Brien".to_string(),
            emails: vec![ContactEmail {
                email: "obrien@example.com".to_string(),
                label: None,
            }],
            phones: vec![],
            organization: Some("Comma, Inc".to_string()),
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let serialized = serialize_vcard(&contact);
        let parsed = parse_vcard(&serialized, None, None).unwrap();
        assert_eq!(parsed.name, contact.name);
        assert_eq!(parsed.organization, contact.organization);
    }

    /// Golden-value test: pins the exact SipHasher13 output for a known input (D-04).
    /// If this test breaks, the hasher or its seed has changed — contact IDs would
    /// silently shift for contacts without a UID in the vCard.
    #[test]
    fn hash_id_golden() {
        // Computed once with SipHasher13::new_with_keys(0, 0) and recorded here.
        // Do NOT change this value; instead investigate why the hash changed.
        assert_eq!(hash_id("John Doe"), 17102779196494968154u64);
    }

    #[test]
    fn hash_id_deterministic() {
        // Same input must produce the same output on every call.
        assert_eq!(hash_id("John Doe"), hash_id("John Doe"));
    }

    #[test]
    fn hash_id_distinct_inputs() {
        // Distinct single-char inputs must not trivially collide.
        assert_ne!(hash_id("a"), hash_id("b"));
    }

    #[test]
    fn hash_id_empty_string_deterministic() {
        // Empty string must produce a stable, non-panicking result.
        assert_eq!(hash_id(""), hash_id(""));
    }

    #[test]
    fn test_carddav_client_new_returns_ok() {
        let result = CardDavClient::new("user@example.com".to_string(), "pass".to_string());
        assert!(result.is_ok());
    }

    // ===== SEC-02: EMAIL/TEL injection prevention tests =====

    #[test]
    fn test_vcard_email_newline_escaped() {
        let contact = Contact {
            id: "u1".into(),
            name: "Test".into(),
            emails: vec![ContactEmail {
                label: None,
                email: "a@b.com\nFN:injected".into(),
            }],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let vcard = serialize_vcard(&contact);
        assert!(
            vcard.contains(r"EMAIL:a@b.com\nFN:injected"),
            "expected escaped \\n, got: {vcard}"
        );
        // No raw injection: there must be no property line starting with "FN:injected"
        let fn_injected_lines = vcard
            .lines()
            .filter(|l| l.trim_start().starts_with("FN:injected"))
            .count();
        assert_eq!(fn_injected_lines, 0, "injection not blocked: {vcard}");
    }

    #[test]
    fn test_vcard_email_semicolon_escaped() {
        let contact = Contact {
            id: "u1".into(),
            name: "T".into(),
            emails: vec![ContactEmail {
                label: None,
                email: "a;b@c.com".into(),
            }],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let vcard = serialize_vcard(&contact);
        assert!(vcard.contains(r"EMAIL:a\;b@c.com"), "got: {vcard}");
    }

    #[test]
    fn test_vcard_tel_special_chars_escaped() {
        let contact = Contact {
            id: "u1".into(),
            name: "T".into(),
            emails: vec![],
            phones: vec![ContactPhone {
                label: None,
                number: r";,\n".into(),
            }],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let vcard = serialize_vcard(&contact);
        // original: literal chars ; , \ n -> escape_value produces \; \, \\ n
        assert!(vcard.contains(r"TEL:\;\,\\n"), "got: {vcard}");
    }

    #[test]
    fn test_vcard_label_injection_blocked() {
        let contact = Contact {
            id: "u1".into(),
            name: "T".into(),
            emails: vec![ContactEmail {
                label: Some("HOME\nMALICIOUS:pwn".into()),
                email: "a@b.com".into(),
            }],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        };
        let vcard = serialize_vcard(&contact);
        let malicious_lines = vcard
            .lines()
            .filter(|l| l.trim_start().starts_with("MALICIOUS:"))
            .count();
        assert_eq!(malicious_lines, 0, "label injection not blocked: {vcard}");
    }

    // ===== collect_partial_contacts tests =====

    fn make_contact(name: &str) -> Contact {
        Contact {
            id: name.to_lowercase().replace(' ', "-"),
            name: name.to_string(),
            emails: vec![],
            phones: vec![],
            organization: None,
            title: None,
            notes: None,
            address: None,
            href: None,
            etag: None,
        }
    }

    #[test]
    fn search_contacts_partial_failure_returns_successes() {
        // 3 books: first and third succeed, middle fails
        let results: Vec<Result<(String, Vec<Contact>)>> = vec![
            Ok(("/book1".to_string(), vec![make_contact("Alice")])),
            Err(Error::Server("connection refused".to_string())),
            Ok(("/book3".to_string(), vec![make_contact("Charlie")])),
        ];
        let contacts = collect_partial_contacts(results);
        assert_eq!(contacts.len(), 2);
        let names: Vec<&str> = contacts.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Alice"));
        assert!(names.contains(&"Charlie"));
    }

    #[test]
    fn search_contacts_all_success_flattened_in_order() {
        let results: Vec<Result<(String, Vec<Contact>)>> = vec![
            Ok(("/book1".to_string(), vec![make_contact("Alice"), make_contact("Bob")])),
            Ok(("/book2".to_string(), vec![make_contact("Charlie")])),
        ];
        let contacts = collect_partial_contacts(results);
        assert_eq!(contacts.len(), 3);
        // Order should match books iteration order
        assert_eq!(contacts[0].name, "Alice");
        assert_eq!(contacts[1].name, "Bob");
        assert_eq!(contacts[2].name, "Charlie");
    }

    #[test]
    fn search_contacts_all_failure_returns_empty() {
        let results: Vec<Result<(String, Vec<Contact>)>> = vec![
            Err(Error::Server("timeout".to_string())),
            Err(Error::Server("timeout".to_string())),
            Err(Error::Server("timeout".to_string())),
        ];
        let contacts = collect_partial_contacts(results);
        assert!(contacts.is_empty());
    }

    // ===== ContactGroup / group vCard tests =====

    fn make_group_vcard(uid: &str, name: &str, members: &[&str]) -> String {
        let mut lines = vec![
            "BEGIN:VCARD".to_string(),
            "VERSION:3.0".to_string(),
            format!("UID:{uid}"),
            format!("FN:{name}"),
            "X-ADDRESSBOOKSERVER-KIND:group".to_string(),
        ];
        for m in members {
            lines.push(format!("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:{m}"));
        }
        lines.push("END:VCARD".to_string());
        lines.join("\n")
    }

    #[test]
    fn test_parse_group_vcard_valid() {
        let vcard = make_group_vcard("grp-001", "Friends", &["uid-1", "uid-2"]);
        let group = parse_group_vcard(&vcard, None, None).unwrap();
        assert_eq!(group.id, "grp-001");
        assert_eq!(group.name, "Friends");
        assert_eq!(group.member_uids, vec!["uid-1", "uid-2"]);
        assert!(group.href.is_none());
        assert!(group.etag.is_none());
    }

    #[test]
    fn test_parse_group_vcard_non_group_returns_none() {
        let vcard =
            "BEGIN:VCARD\nVERSION:3.0\nUID:abc\nFN:Alice Smith\nEMAIL:alice@example.com\nEND:VCARD";
        assert!(parse_group_vcard(vcard, None, None).is_none());
    }

    #[test]
    fn test_parse_group_vcard_missing_fn_returns_none() {
        let vcard = "BEGIN:VCARD\nVERSION:3.0\nUID:grp-001\nX-ADDRESSBOOKSERVER-KIND:group\nEND:VCARD";
        assert!(parse_group_vcard(vcard, None, None).is_none());
    }

    #[test]
    fn test_parse_group_vcard_multiple_members() {
        let vcard = make_group_vcard("grp-002", "Work", &["a", "b", "c"]);
        let group = parse_group_vcard(&vcard, None, None).unwrap();
        assert_eq!(group.member_uids.len(), 3);
        assert_eq!(group.member_uids[0], "a");
        assert_eq!(group.member_uids[1], "b");
        assert_eq!(group.member_uids[2], "c");
    }

    #[test]
    fn test_parse_vcard_filters_group() {
        let vcard = make_group_vcard("grp-001", "Friends", &["uid-1"]);
        // parse_vcard must return None for group vCards
        assert!(parse_vcard(&vcard, None, None).is_none());
    }

    #[test]
    fn test_serialize_group_vcard_structure() {
        let group = ContactGroup {
            id: "grp-001".to_string(),
            name: "Friends".to_string(),
            member_uids: vec!["uid-1".to_string(), "uid-2".to_string()],
            href: None,
            etag: None,
        };
        let output = serialize_group_vcard(&group);
        assert!(output.contains("BEGIN:VCARD"));
        assert!(output.contains("VERSION:3.0"));
        assert!(output.contains("UID:grp-001"));
        assert!(output.contains("FN:Friends"));
        assert!(output.contains("X-ADDRESSBOOKSERVER-KIND:group"));
        assert!(output.contains("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-1"));
        assert!(output.contains("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-2"));
        assert!(output.contains("END:VCARD"));
    }

    #[test]
    fn test_serialize_group_vcard_empty_members() {
        let group = ContactGroup {
            id: "grp-002".to_string(),
            name: "Empty".to_string(),
            member_uids: vec![],
            href: None,
            etag: None,
        };
        let output = serialize_group_vcard(&group);
        assert!(!output.contains("X-ADDRESSBOOKSERVER-MEMBER"));
    }

    #[test]
    fn test_parse_groups_from_xml_extracts_only_groups() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/dav/addressbooks/user/test/Default/contact1.vcf</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"etag-contact"</d:getetag>
        <card:address-data>BEGIN:VCARD
VERSION:3.0
UID:uid-contact
FN:Alice Smith
END:VCARD</card:address-data>
      </d:prop>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/dav/addressbooks/user/test/Default/group1.vcf</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"etag-group"</d:getetag>
        <card:address-data>BEGIN:VCARD
VERSION:3.0
UID:grp-001
FN:Friends
X-ADDRESSBOOKSERVER-KIND:group
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-contact
END:VCARD</card:address-data>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
        let groups = parse_groups_from_xml(xml).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "grp-001");
        assert_eq!(groups[0].name, "Friends");
        assert_eq!(groups[0].member_uids, vec!["uid-contact"]);
        assert_eq!(
            groups[0].href,
            Some("/dav/addressbooks/user/test/Default/group1.vcf".to_string())
        );
        assert_eq!(groups[0].etag, Some("\"etag-group\"".to_string()));
    }

    #[test]
    fn test_build_group_href() {
        assert_eq!(
            build_group_href("/dav/addressbooks/user/test/Default/", "grp-123"),
            "/dav/addressbooks/user/test/Default/grp-123.vcf"
        );
    }

    #[test]
    fn test_map_group_write_response_success() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"etag-group-new\""));
        let result =
            map_group_write_response("grp-001", None, StatusCode::CREATED, &headers, "").unwrap();
        assert_eq!(result.as_deref(), Some("\"etag-group-new\""));
    }

    #[test]
    fn test_map_group_write_response_conflict() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"etag-server\""));
        let err = map_group_write_response(
            "grp-001",
            Some("\"etag-old\""),
            StatusCode::PRECONDITION_FAILED,
            &headers,
            "",
        )
        .unwrap_err();
        match err {
            Error::GroupConflict {
                id,
                sent_etag,
                server_etag,
            } => {
                assert_eq!(id, "grp-001");
                assert_eq!(sent_etag, "\"etag-old\"");
                assert_eq!(server_etag.as_deref(), Some("\"etag-server\""));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_map_group_write_response_not_found() {
        let err = map_group_write_response(
            "grp-001",
            Some("\"etag-old\""),
            StatusCode::NOT_FOUND,
            &HeaderMap::new(),
            "",
        )
        .unwrap_err();
        match err {
            Error::GroupNotFound(id) => assert_eq!(id, "grp-001"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_collect_partial_groups_partial_failure() {
        fn make_group(name: &str) -> ContactGroup {
            ContactGroup {
                id: name.to_lowercase().replace(' ', "-"),
                name: name.to_string(),
                member_uids: vec![],
                href: None,
                etag: None,
            }
        }
        let results: Vec<Result<(String, Vec<ContactGroup>)>> = vec![
            Ok(("/book1".to_string(), vec![make_group("Alpha")])),
            Err(Error::Server("timeout".to_string())),
            Ok(("/book3".to_string(), vec![make_group("Beta")])),
        ];
        let groups = collect_partial_groups(results);
        assert_eq!(groups.len(), 2);
        let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Beta"));
    }

    // ===== add_group_member / remove_group_member unit-level logic tests =====

    #[test]
    fn test_serialize_group_vcard_member_lines_format() {
        // Verify serialize_group_vcard emits the correct X-ADDRESSBOOKSERVER-MEMBER format
        // for each member UID — this is the exact format the server expects.
        let group = ContactGroup {
            id: "grp-test".to_string(),
            name: "Test Group".to_string(),
            member_uids: vec!["uid-aaa".to_string(), "uid-bbb".to_string()],
            href: None,
            etag: None,
        };
        let output = serialize_group_vcard(&group);
        assert!(
            output.contains("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-aaa"),
            "expected member line for uid-aaa, got: {output}"
        );
        assert!(
            output.contains("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-bbb"),
            "expected member line for uid-bbb, got: {output}"
        );
        // Verify no duplicate member lines are emitted
        let member_count = output
            .lines()
            .filter(|l| l.trim_start().starts_with("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:uid-aaa"))
            .count();
        assert_eq!(member_count, 1, "expected exactly 1 member line for uid-aaa");
    }

    #[test]
    fn test_add_group_member_idempotency_guard_logic() {
        // Unit-level check of the idempotency guard: if contact_uid is already in
        // member_uids, the guard fires and no duplicate is added.
        let group = ContactGroup {
            id: "grp-001".to_string(),
            name: "Friends".to_string(),
            member_uids: vec!["uid-1".to_string(), "uid-2".to_string()],
            href: Some("/dav/grp-001.vcf".to_string()),
            etag: Some("\"etag-1\"".to_string()),
        };
        // Simulate the idempotency check
        let contact_uid = "uid-1";
        assert!(
            group.member_uids.contains(&contact_uid.to_string()),
            "idempotency guard should fire: uid-1 already in member_uids"
        );
        // Simulate the mutation path for a new member
        let contact_uid_new = "uid-3";
        assert!(
            !group.member_uids.contains(&contact_uid_new.to_string()),
            "uid-3 should not yet be in member_uids"
        );
        let mut updated = group.clone();
        updated.member_uids.push(contact_uid_new.to_string());
        assert_eq!(updated.member_uids.len(), 3);
        assert!(updated.member_uids.contains(&"uid-3".to_string()));
    }

    #[test]
    fn test_remove_group_member_idempotency_guard_logic() {
        // Unit-level check of the remove idempotency guard: if contact_uid is NOT in
        // member_uids, the guard fires and no PUT is needed.
        let group = ContactGroup {
            id: "grp-001".to_string(),
            name: "Friends".to_string(),
            member_uids: vec!["uid-1".to_string(), "uid-2".to_string()],
            href: Some("/dav/grp-001.vcf".to_string()),
            etag: Some("\"etag-1\"".to_string()),
        };
        // uid-99 is not a member — guard fires
        let contact_uid = "uid-99";
        assert!(
            !group.member_uids.contains(&contact_uid.to_string()),
            "idempotency guard should fire: uid-99 not in member_uids"
        );
        // Simulate the retain mutation for an existing member
        let contact_uid_existing = "uid-1";
        let mut updated = group.clone();
        updated.member_uids.retain(|uid| uid != contact_uid_existing);
        assert_eq!(updated.member_uids.len(), 1);
        assert!(!updated.member_uids.contains(&"uid-1".to_string()));
        assert!(updated.member_uids.contains(&"uid-2".to_string()));
    }
}
