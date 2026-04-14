use anyhow::Result as AnyResult;

use crate::carddav::{
    CardDavClient, Contact, ContactCreateResult, ContactEmail, ContactGroup, ContactPhone, Uuid,
};
use crate::config::Config;
use crate::models::Output;

#[derive(Debug, Clone, Default)]
pub struct ContactInput {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ContactPatch {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

fn contact_client() -> crate::error::Result<CardDavClient> {
    let config = Config::load()?;
    let username = config.get_username()?;
    let app_password = config.get_app_password()?;
    CardDavClient::new(username, app_password)
}

pub fn build_contact(input: ContactInput) -> Contact {
    Contact {
        id: Uuid::new_v4().to_string(),
        name: input.name,
        emails: input
            .email
            .into_iter()
            .map(|email| ContactEmail { email, label: None })
            .collect(),
        phones: input
            .phone
            .into_iter()
            .map(|number| ContactPhone {
                number,
                label: None,
            })
            .collect(),
        organization: input.organization,
        title: input.title,
        notes: input.notes,
        address: input.address,
        href: None,
        etag: None,
    }
}

pub fn apply_contact_patch(existing: &Contact, patch: ContactPatch) -> Contact {
    Contact {
        id: existing.id.clone(),
        name: patch.name.unwrap_or_else(|| existing.name.clone()),
        emails: match patch.email {
            Some(email) => vec![ContactEmail { email, label: None }],
            None => existing.emails.clone(),
        },
        phones: match patch.phone {
            Some(number) => vec![ContactPhone {
                number,
                label: None,
            }],
            None => existing.phones.clone(),
        },
        organization: patch.organization.or_else(|| existing.organization.clone()),
        title: patch.title.or_else(|| existing.title.clone()),
        notes: patch.notes.or_else(|| existing.notes.clone()),
        address: patch.address.or_else(|| existing.address.clone()),
        href: existing.href.clone(),
        etag: existing.etag.clone(),
    }
}

pub async fn create_contact_record(input: ContactInput) -> crate::error::Result<Contact> {
    let client = contact_client()?;
    let mut contact = build_contact(input);
    let addressbook_href = client.default_addressbook_href().await?;
    let ContactCreateResult { href, etag } =
        client.create_contact(&addressbook_href, &contact).await?;
    contact.href = Some(href);
    contact.etag = etag;
    Ok(contact)
}

pub async fn update_contact_record(
    contact_id: &str,
    patch: ContactPatch,
) -> crate::error::Result<Contact> {
    let client = contact_client()?;
    let existing = client.get_contact_by_id(contact_id).await?;
    let href = existing
        .href
        .clone()
        .ok_or_else(|| crate::error::Error::ContactNotFound(contact_id.to_string()))?;
    let etag = existing
        .etag
        .clone()
        .ok_or_else(|| crate::error::Error::ContactConflict {
            id: contact_id.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })?;
    let mut updated = apply_contact_patch(&existing, patch);
    let new_etag = client.update_contact(&href, &etag, &updated).await?;
    updated.href = Some(href);
    updated.etag = Some(new_etag);
    Ok(updated)
}

pub async fn delete_contact_record(contact_id: &str) -> crate::error::Result<()> {
    let client = contact_client()?;
    let existing = client.get_contact_by_id(contact_id).await?;
    let href = existing
        .href
        .as_deref()
        .ok_or_else(|| crate::error::Error::ContactNotFound(contact_id.to_string()))?;
    let etag = existing
        .etag
        .as_deref()
        .ok_or_else(|| crate::error::Error::ContactConflict {
            id: contact_id.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })?;
    client.delete_contact(href, etag, contact_id).await
}

/// List all contacts from all address books
pub async fn list_contacts() -> AnyResult<()> {
    let client = contact_client()?;

    let addressbooks = client.list_addressbooks().await?;
    eprintln!("Found {} address book(s)", addressbooks.len());

    let mut all_contacts = Vec::new();
    for ab in &addressbooks {
        eprintln!("Fetching from: {}", ab.name);
        let contacts = client.list_contacts(&ab.href).await?;
        all_contacts.extend(contacts);
    }

    Output::success(all_contacts).print();
    Ok(())
}

/// Search contacts by name or email
pub async fn search_contacts(query: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let contacts = client.search_contacts(query).await?;

    Output::success(contacts).print();
    Ok(())
}

pub async fn create_contact(input: ContactInput) -> AnyResult<()> {
    let contact = create_contact_record(input).await?;
    Output {
        success: true,
        data: Some(contact),
        error: None,
        message: Some("Contact created".to_string()),
    }
    .print();
    Ok(())
}

pub async fn update_contact(contact_id: &str, patch: ContactPatch) -> AnyResult<()> {
    let contact = update_contact_record(contact_id, patch).await?;
    Output {
        success: true,
        data: Some(contact),
        error: None,
        message: Some(format!("Contact {} updated", contact_id)),
    }
    .print();
    Ok(())
}

pub async fn delete_contact(contact_id: &str) -> AnyResult<()> {
    delete_contact_record(contact_id).await?;
    Output::<()> {
        success: true,
        data: None,
        error: None,
        message: Some(format!("Contact {} deleted", contact_id)),
    }
    .print();
    Ok(())
}

/// Resolve a group by ID first, then by name if ID lookup returns GroupNotFound.
async fn resolve_group(
    client: &CardDavClient,
    id_or_name: &str,
) -> crate::error::Result<ContactGroup> {
    match client.get_group_by_id(id_or_name).await {
        Ok(group) => return Ok(group),
        Err(crate::error::Error::GroupNotFound(_)) => {}
        Err(e) => return Err(e),
    }
    client.get_group_by_name(id_or_name).await
}

/// List all contact groups across all address books
pub async fn list_groups() -> AnyResult<()> {
    let client = contact_client()?;
    let groups = client.list_groups().await?;
    Output::success(groups).print();
    Ok(())
}

/// Create a new empty contact group
pub async fn create_group(name: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let mut group = ContactGroup {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        member_uids: vec![],
        href: None,
        etag: None,
    };
    let addressbook_href = client.default_addressbook_href().await?;
    let ContactCreateResult { href, etag } = client.create_group(&addressbook_href, &group).await?;
    group.href = Some(href);
    group.etag = etag;
    Output {
        success: true,
        data: Some(group),
        error: None,
        message: Some("Group created".to_string()),
    }
    .print();
    Ok(())
}

/// Get a contact group by ID or name, with resolved member contacts
pub async fn get_group(id_or_name: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let group = resolve_group(&client, id_or_name).await?;
    let members = client.resolve_group_members(&group).await?;
    let data = serde_json::json!({
        "id": group.id,
        "name": group.name,
        "href": group.href,
        "etag": group.etag,
        "member_count": group.member_uids.len(),
        "members": members,
    });
    Output::success(data).print();
    Ok(())
}

/// Rename a contact group
pub async fn rename_group(id_or_name: &str, new_name: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let group = resolve_group(&client, id_or_name).await?;
    let href = group
        .href
        .clone()
        .ok_or_else(|| crate::error::Error::GroupNotFound(id_or_name.to_string()))?;
    let etag = group
        .etag
        .clone()
        .ok_or_else(|| crate::error::Error::GroupConflict {
            id: id_or_name.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })?;
    let new_etag = client.rename_group(&href, &etag, &group, new_name).await?;
    Output::<()> {
        success: true,
        data: None,
        error: None,
        message: Some(format!("Group {} renamed to {}: etag={}", group.id, new_name, new_etag)),
    }
    .print();
    Ok(())
}

/// Delete a contact group (members are NOT deleted)
pub async fn delete_group(id_or_name: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let group = resolve_group(&client, id_or_name).await?;
    let href = group
        .href
        .clone()
        .ok_or_else(|| crate::error::Error::GroupNotFound(id_or_name.to_string()))?;
    let etag = group
        .etag
        .clone()
        .ok_or_else(|| crate::error::Error::GroupConflict {
            id: id_or_name.to_string(),
            sent_etag: String::new(),
            server_etag: None,
        })?;
    client.delete_group(&href, &etag, &group.id).await?;
    Output::<()> {
        success: true,
        data: None,
        error: None,
        message: Some(format!("Group {} deleted", group.id)),
    }
    .print();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contact() -> Contact {
        Contact {
            id: "contact-1".to_string(),
            name: "Jane Doe".to_string(),
            emails: vec![ContactEmail {
                email: "jane@example.com".to_string(),
                label: None,
            }],
            phones: vec![ContactPhone {
                number: "+1 555 0000".to_string(),
                label: None,
            }],
            organization: Some("Acme".to_string()),
            title: Some("Manager".to_string()),
            notes: Some("Met at conference".to_string()),
            address: Some("123 Main St".to_string()),
            href: Some("/dav/addressbooks/user/test/default/contact-1.vcf".to_string()),
            etag: Some("\"etag-1\"".to_string()),
        }
    }

    #[test]
    fn test_build_contact_uses_uuid_and_optional_fields() {
        let contact = build_contact(ContactInput {
            name: "Jane Doe".to_string(),
            email: Some("jane@example.com".to_string()),
            phone: Some("+1 555 0000".to_string()),
            organization: Some("Acme".to_string()),
            title: Some("Manager".to_string()),
            address: Some("123 Main St".to_string()),
            notes: Some("Met at conference".to_string()),
        });

        assert_eq!(contact.name, "Jane Doe");
        assert_eq!(contact.emails[0].email, "jane@example.com");
        assert_eq!(contact.phones[0].number, "+1 555 0000");
        assert_eq!(contact.organization.as_deref(), Some("Acme"));
        assert_eq!(contact.title.as_deref(), Some("Manager"));
        assert_eq!(contact.address.as_deref(), Some("123 Main St"));
        assert_eq!(contact.notes.as_deref(), Some("Met at conference"));
        assert!(Uuid::parse_str(&contact.id).is_ok());
    }

    #[test]
    fn test_apply_contact_patch_only_overrides_supplied_fields() {
        let existing = sample_contact();
        let updated = apply_contact_patch(
            &existing,
            ContactPatch {
                phone: Some("+1 555 0100".to_string()),
                notes: Some("Updated note".to_string()),
                ..Default::default()
            },
        );

        assert_eq!(updated.name, existing.name);
        assert_eq!(updated.emails, existing.emails);
        assert_eq!(updated.phones[0].number, "+1 555 0100");
        assert_eq!(updated.notes.as_deref(), Some("Updated note"));
        assert_eq!(updated.organization, existing.organization);
        assert_eq!(updated.href, existing.href);
        assert_eq!(updated.etag, existing.etag);
    }
}
