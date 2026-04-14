//! GraphQL mutation resolvers

use async_graphql::{Context, Object, Result};

use crate::carddav::{ContactGroup, Uuid};
use crate::models::EmailAddress;
use crate::util::parse_addresses;
use crate::{commands, error::Error};

use super::types::*;

pub struct MutationRoot;

#[Object]
#[allow(clippy::too_many_arguments)]
impl MutationRoot {
    async fn create_contact(
        &self,
        #[graphql(desc = "Full name")] name: String,
        #[graphql(desc = "Primary email address")] email: Option<String>,
        #[graphql(desc = "Primary phone number")] phone: Option<String>,
        #[graphql(desc = "Organization / company")] organization: Option<String>,
        #[graphql(desc = "Job title")] title: Option<String>,
        #[graphql(desc = "Street address")] address: Option<String>,
        #[graphql(desc = "Notes")] notes: Option<String>,
    ) -> Result<GqlContactMutationResult> {
        match commands::create_contact_record(commands::ContactInput {
            name,
            email,
            phone,
            organization,
            title,
            address,
            notes,
        })
        .await
        {
            Ok(contact) => Ok(GqlContactMutationResult {
                success: true,
                contact: Some(GqlContact::from(contact)),
                message: Some("Contact created".to_string()),
                error: None,
            }),
            Err(error) => Ok(GqlContactMutationResult {
                success: false,
                contact: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn update_contact(
        &self,
        #[graphql(desc = "Contact ID")] id: String,
        #[graphql(desc = "Updated full name")] name: Option<String>,
        #[graphql(desc = "Updated primary email address")] email: Option<String>,
        #[graphql(desc = "Updated primary phone number")] phone: Option<String>,
        #[graphql(desc = "Updated organization / company")] organization: Option<String>,
        #[graphql(desc = "Updated job title")] title: Option<String>,
        #[graphql(desc = "Updated street address")] address: Option<String>,
        #[graphql(desc = "Updated notes")] notes: Option<String>,
    ) -> Result<GqlContactMutationResult> {
        match commands::update_contact_record(
            &id,
            commands::ContactPatch {
                name,
                email,
                phone,
                organization,
                title,
                address,
                notes,
            },
        )
        .await
        {
            Ok(contact) => Ok(GqlContactMutationResult {
                success: true,
                contact: Some(GqlContact::from(contact)),
                message: Some(format!("Contact {} updated", id)),
                error: None,
            }),
            Err(error) => Ok(GqlContactMutationResult {
                success: false,
                contact: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn delete_contact(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM with the token from preview")]
        action: ContactDeleteAction,
        #[graphql(desc = "Contact ID")] id: String,
        #[graphql(desc = "Confirmation token returned by PREVIEW")] confirmation_token: Option<
            String,
        >,
    ) -> Result<GqlContactDeleteResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&id]);

        if matches!(action, ContactDeleteAction::Preview) {
            return Ok(GqlContactDeleteResult {
                success: true,
                deleted_id: None,
                preview: Some(format!(
                    "Delete contact {}. Re-run deleteContact with action=CONFIRM and the confirmation token to proceed.",
                    id
                )),
                confirmation_token: Some(token),
                message: None,
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlContactDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        match commands::delete_contact_record(&id).await {
            Ok(()) => Ok(GqlContactDeleteResult {
                success: true,
                deleted_id: Some(id.clone()),
                preview: None,
                confirmation_token: None,
                message: Some(format!("Contact {} deleted", id)),
                error: None,
            }),
            Err(Error::ContactNotFound(_)) => Ok(GqlContactDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(format!("Contact {} not found", id)),
            }),
            Err(error) => Ok(GqlContactDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn create_calendar(
        &self,
        #[graphql(desc = "Calendar display name")] name: String,
        #[graphql(desc = "Optional calendar color, e.g. #3a87ad")] color: Option<String>,
    ) -> Result<GqlCalendarMutationResult> {
        match commands::create_calendar_record(&name, color.as_deref()).await {
            Ok(calendar) => Ok(GqlCalendarMutationResult {
                success: true,
                calendar: Some(calendar.into()),
                message: Some("Calendar created".to_string()),
                error: None,
            }),
            Err(error) => Ok(GqlCalendarMutationResult {
                success: false,
                calendar: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn update_calendar(
        &self,
        #[graphql(desc = "Calendar ID")] id: String,
        #[graphql(desc = "Updated display name")] name: Option<String>,
        #[graphql(desc = "Updated calendar color")] color: Option<String>,
    ) -> Result<GqlCalendarMutationResult> {
        match commands::update_calendar_record(&id, name.as_deref(), color.as_deref()).await {
            Ok(calendar) => Ok(GqlCalendarMutationResult {
                success: true,
                calendar: Some(calendar.into()),
                message: Some(format!("Calendar {} updated", id)),
                error: None,
            }),
            Err(error) => Ok(GqlCalendarMutationResult {
                success: false,
                calendar: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn delete_calendar(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM with the token from preview")]
        action: CalendarDeleteAction,
        #[graphql(desc = "Calendar ID")] id: String,
        #[graphql(desc = "Confirmation token returned by PREVIEW")] confirmation_token: Option<
            String,
        >,
    ) -> Result<GqlCalendarDeleteResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&id]);
        if matches!(action, CalendarDeleteAction::Preview) {
            return Ok(GqlCalendarDeleteResult {
                success: true,
                deleted_id: None,
                preview: Some(format!(
                    "Delete calendar {}. Re-run deleteCalendar with action=CONFIRM and the confirmation token to proceed.",
                    id
                )),
                confirmation_token: Some(token),
                message: None,
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlCalendarDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        match commands::delete_calendar_record(&id).await {
            Ok(()) => Ok(GqlCalendarDeleteResult {
                success: true,
                deleted_id: Some(id.clone()),
                preview: None,
                confirmation_token: None,
                message: Some(format!("Calendar {} deleted", id)),
                error: None,
            }),
            Err(error) => Ok(GqlCalendarDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    /// Create a new empty contact group.
    async fn create_group(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Group name")] name: String,
    ) -> Result<GqlGroupMutationResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let carddav = app_ctx.get_carddav().await?;
        let group = ContactGroup {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
            member_uids: vec![],
            href: None,
            etag: None,
        };
        let addressbook_href = carddav
            .default_addressbook_href()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let result = carddav
            .create_group(&addressbook_href, &group)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let created = ContactGroup {
            href: Some(result.href),
            etag: result.etag,
            ..group
        };
        Ok(GqlGroupMutationResult {
            success: true,
            group: Some(GqlContactGroup::from(created)),
            message: Some(format!("Group '{}' created", name)),
            error: None,
        })
    }

    /// Rename a contact group.
    async fn rename_group(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Group ID (UID)")] id: String,
        #[graphql(desc = "New group name")] new_name: String,
    ) -> Result<GqlGroupMutationResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let carddav = app_ctx.get_carddav().await?;
        let group = carddav
            .get_group_by_id(&id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let href = group
            .href
            .as_deref()
            .ok_or_else(|| async_graphql::Error::new(format!("Group {} has no href", id)))?;
        let etag = group
            .etag
            .as_deref()
            .ok_or_else(|| async_graphql::Error::new(format!("Group {} has no etag", id)))?;
        let new_etag = carddav
            .rename_group(href, etag, &group, &new_name)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let renamed = ContactGroup {
            name: new_name.clone(),
            etag: Some(new_etag),
            ..group
        };
        Ok(GqlGroupMutationResult {
            success: true,
            group: Some(GqlContactGroup::from(renamed)),
            message: Some(format!("Group renamed to '{}'", new_name)),
            error: None,
        })
    }

    /// Delete a contact group (members are NOT deleted). Use PREVIEW first to get a confirmation token.
    async fn delete_group(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW to get token, CONFIRM to delete")] action: GroupDeleteAction,
        #[graphql(desc = "Group ID (UID)")] id: String,
        #[graphql(desc = "Confirmation token returned by PREVIEW")] confirmation_token: Option<
            String,
        >,
    ) -> Result<GqlGroupDeleteResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&["delete_group", &id]);

        match action {
            GroupDeleteAction::Preview => {
                let carddav = app_ctx.get_carddav().await?;
                let group = carddav
                    .get_group_by_id(&id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(GqlGroupDeleteResult {
                    success: true,
                    deleted_id: None,
                    preview: Some(format!(
                        "Will delete group '{}' ({}). Members will NOT be deleted.",
                        group.name, id
                    )),
                    confirmation_token: Some(token),
                    message: None,
                    error: None,
                })
            }
            GroupDeleteAction::Confirm => {
                if confirmation_token.as_deref() != Some(&token) {
                    return Ok(GqlGroupDeleteResult {
                        success: false,
                        deleted_id: None,
                        preview: None,
                        confirmation_token: None,
                        message: None,
                        error: Some(
                            "Missing or invalid confirmation_token. Use action=PREVIEW first."
                                .to_string(),
                        ),
                    });
                }
                let carddav = app_ctx.get_carddav().await?;
                let group = carddav
                    .get_group_by_id(&id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let href = group
                    .href
                    .as_deref()
                    .ok_or_else(|| async_graphql::Error::new(format!("Group {} has no href", id)))?;
                let etag = group
                    .etag
                    .as_deref()
                    .ok_or_else(|| async_graphql::Error::new(format!("Group {} has no etag", id)))?;
                carddav
                    .delete_group(href, etag, &id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(GqlGroupDeleteResult {
                    success: true,
                    deleted_id: Some(id),
                    preview: None,
                    confirmation_token: None,
                    message: Some("Group deleted".to_string()),
                    error: None,
                })
            }
        }
    }

    async fn create_event(
        &self,
        #[graphql(
            desc = "Optional calendar ID; defaults to the account's primary/default calendar"
        )]
        calendar_id: Option<String>,
        #[graphql(desc = "Event title")] title: String,
        #[graphql(desc = "Start: YYYY-MM-DD, YYYY-MM-DDTHH:MM[:SS], or RFC3339")] start: String,
        #[graphql(desc = "End: YYYY-MM-DD, YYYY-MM-DDTHH:MM[:SS], or RFC3339")] end: String,
        #[graphql(desc = "Timezone for naive local datetimes")] timezone: Option<String>,
        #[graphql(desc = "Location")] location: Option<String>,
        #[graphql(desc = "Description")] description: Option<String>,
        #[graphql(desc = "Attendees")] attendees: Option<Vec<GqlEventAttendeeInput>>,
        #[graphql(desc = "Recurrence rule")] recurrence: Option<GqlEventRecurrenceInput>,
        #[graphql(desc = "Reminders")] reminders: Option<Vec<GqlEventReminderInput>>,
    ) -> Result<GqlEventMutationResult> {
        match commands::create_event_record(commands::EventInput {
            calendar_id,
            title,
            start,
            end,
            timezone,
            location,
            description,
            attendees: attendees
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            recurrence: recurrence.map(Into::into),
            reminders: reminders
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
        })
        .await
        {
            Ok(event) => Ok(GqlEventMutationResult {
                success: true,
                event: Some(event.into()),
                message: Some("Event created".to_string()),
                error: None,
            }),
            Err(error) => Ok(GqlEventMutationResult {
                success: false,
                event: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn update_event(
        &self,
        #[graphql(desc = "Event UID")] id: String,
        #[graphql(desc = "Optional calendar ID hint")] calendar_id: Option<String>,
        #[graphql(desc = "Updated title")] title: Option<String>,
        #[graphql(desc = "Updated start")] start: Option<String>,
        #[graphql(desc = "Updated end")] end: Option<String>,
        #[graphql(desc = "Updated timezone")] timezone: Option<String>,
        #[graphql(desc = "Updated location. Pass an empty string to clear it.")] location: Option<
            String,
        >,
        #[graphql(desc = "Updated description. Pass an empty string to clear it.")]
        description: Option<String>,
        #[graphql(desc = "Replace attendees with this set")] attendees: Option<
            Vec<GqlEventAttendeeInput>,
        >,
        #[graphql(desc = "Replace recurrence with this rule")] recurrence: Option<
            GqlEventRecurrenceInput,
        >,
        #[graphql(desc = "Clear recurrence entirely")] clear_recurrence: Option<bool>,
        #[graphql(desc = "Replace reminders with this set")] reminders: Option<
            Vec<GqlEventReminderInput>,
        >,
        #[graphql(desc = "Clear reminders entirely")] clear_reminders: Option<bool>,
    ) -> Result<GqlEventMutationResult> {
        match commands::update_event_record(
            &id,
            calendar_id.as_deref(),
            commands::EventPatch {
                title,
                start,
                end,
                timezone,
                location,
                description,
                attendees: attendees.map(|items| items.into_iter().map(Into::into).collect()),
                recurrence: recurrence.map(Into::into),
                clear_recurrence: clear_recurrence.unwrap_or(false),
                reminders: reminders.map(|items| items.into_iter().map(Into::into).collect()),
                clear_reminders: clear_reminders.unwrap_or(false),
            },
        )
        .await
        {
            Ok(event) => Ok(GqlEventMutationResult {
                success: true,
                event: Some(event.into()),
                message: Some(format!("Event {} updated", id)),
                error: None,
            }),
            Err(error) => Ok(GqlEventMutationResult {
                success: false,
                event: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    async fn delete_event(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM with the token from preview")]
        action: EventDeleteAction,
        #[graphql(desc = "Event UID")] id: String,
        #[graphql(desc = "Optional calendar ID hint")] calendar_id: Option<String>,
        #[graphql(desc = "Confirmation token returned by PREVIEW")] confirmation_token: Option<
            String,
        >,
    ) -> Result<GqlEventDeleteResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&id, calendar_id.as_deref().unwrap_or("")]);
        if matches!(action, EventDeleteAction::Preview) {
            return Ok(GqlEventDeleteResult {
                success: true,
                deleted_id: None,
                preview: Some(format!(
                    "Delete event {}. Re-run deleteEvent with action=CONFIRM and the confirmation token to proceed.",
                    id
                )),
                confirmation_token: Some(token),
                message: None,
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlEventDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        match commands::delete_event_record(&id, calendar_id.as_deref()).await {
            Ok(()) => Ok(GqlEventDeleteResult {
                success: true,
                deleted_id: Some(id.clone()),
                preview: None,
                confirmation_token: None,
                message: Some(format!("Event {} deleted", id)),
                error: None,
            }),
            Err(error) => Ok(GqlEventDeleteResult {
                success: false,
                deleted_id: None,
                preview: None,
                confirmation_token: None,
                message: None,
                error: Some(error.to_string()),
            }),
        }
    }

    /// Compose and send a new email. ALWAYS use action=PREVIEW first, show the user, then CONFIRM or DRAFT with the confirmation_token from the preview.
    async fn send_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM to send or DRAFT to save")]
        action: SendAction,
        #[graphql(desc = "Recipient email address(es), comma-separated")] to: String,
        #[graphql(desc = "Email subject line")] subject: String,
        #[graphql(desc = "Email body text")] body: String,
        #[graphql(desc = "CC recipients, comma-separated")] cc: Option<String>,
        #[graphql(desc = "BCC recipients (hidden), comma-separated")] bcc: Option<String>,
        #[graphql(desc = "Send from a specific identity/email address")] from: Option<String>,
        #[graphql(desc = "Token from PREVIEW response — required for CONFIRM/DRAFT")]
        confirmation_token: Option<String>,
    ) -> Result<GqlComposeResult> {
        let to_addrs = parse_addresses(&to);
        let cc_addrs = cc.as_deref().map(parse_addresses).unwrap_or_default();
        let bcc_addrs = bcc.as_deref().map(parse_addresses).unwrap_or_default();
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&to, &subject, &body]);

        if matches!(action, SendAction::Preview) {
            return Ok(GqlComposeResult {
                success: true,
                email_id: None,
                preview: Some(format_send_preview(
                    &to_addrs, &cc_addrs, &bcc_addrs, &subject, &body,
                )),
                confirmation_token: Some(token),
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first to get the token."
                        .to_string(),
                ),
            });
        }

        let draft = matches!(action, SendAction::Draft);
        let client = app_ctx.require_jmap()?;
        let mut client = client.lock().await;

        match client
            .send_email(
                to_addrs,
                &subject,
                &body,
                None,
                crate::jmap::ComposeParams {
                    cc: cc_addrs,
                    bcc: bcc_addrs,
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        {
            Ok(email_id) => Ok(GqlComposeResult {
                success: true,
                email_id: Some(email_id),
                preview: None,
                confirmation_token: None,
                error: None,
            }),
            Err(e) => Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Reply to an existing email thread. ALWAYS use action=PREVIEW first.
    async fn reply_to_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM to send or DRAFT to save")]
        action: SendAction,
        #[graphql(desc = "The email ID to reply to")] email_id: String,
        #[graphql(desc = "Reply body text (your response, without quoting original)")] body: String,
        #[graphql(desc = "Reply to all recipients")] all: Option<bool>,
        #[graphql(desc = "CC recipients, comma-separated")] cc: Option<String>,
        #[graphql(desc = "BCC recipients, comma-separated")] bcc: Option<String>,
        #[graphql(desc = "Send from a specific identity/email address")] from: Option<String>,
        #[graphql(desc = "Token from PREVIEW response — required for CONFIRM/DRAFT")]
        confirmation_token: Option<String>,
    ) -> Result<GqlComposeResult> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&email_id, &body]);
        let client = app_ctx.require_jmap()?;
        let mut client = client.lock().await;

        let original = client.get_email(&email_id).await?;
        let reply_all = all.unwrap_or(false);
        let cc_addrs = cc.as_deref().map(parse_addresses).unwrap_or_default();
        let bcc_addrs = bcc.as_deref().map(parse_addresses).unwrap_or_default();

        let subject = if original
            .subject
            .as_ref()
            .is_some_and(|s| s.to_lowercase().starts_with("re:"))
        {
            original.subject.clone().unwrap_or_default()
        } else {
            format!("Re: {}", original.subject.as_deref().unwrap_or(""))
        };

        let to_addrs: Vec<EmailAddress> = original.from.clone().unwrap_or_default();

        if matches!(action, SendAction::Preview) {
            let in_reply_to = original
                .message_id
                .as_ref()
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_else(|| "(none)".to_string());
            return Ok(GqlComposeResult {
                success: true,
                email_id: None,
                preview: Some(format!(
                    "REPLY PREVIEW:\nTo: {}\nCC: {}\nBCC: {}\nSubject: {}\nIn-Reply-To: {}\n\n--- Your Reply ---\n{}",
                    format_addrs(&to_addrs),
                    if cc_addrs.is_empty() {
                        "(none)".to_string()
                    } else {
                        format_addrs(&cc_addrs)
                    },
                    if bcc_addrs.is_empty() {
                        "(none)".to_string()
                    } else {
                        format_addrs(&bcc_addrs)
                    },
                    subject,
                    in_reply_to,
                    body
                )),
                confirmation_token: Some(token),
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        let draft = matches!(action, SendAction::Draft);
        match client
            .reply_email(
                &original,
                &body,
                reply_all,
                crate::jmap::ComposeParams {
                    cc: cc_addrs,
                    bcc: bcc_addrs,
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        {
            Ok(eid) => Ok(GqlComposeResult {
                success: true,
                email_id: Some(eid),
                preview: None,
                confirmation_token: None,
                error: None,
            }),
            Err(e) => Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Forward an email to new recipients. ALWAYS use action=PREVIEW first.
    async fn forward_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "PREVIEW first, then CONFIRM to send or DRAFT to save")]
        action: SendAction,
        #[graphql(desc = "The email ID to forward")] email_id: String,
        #[graphql(desc = "Recipient email address(es), comma-separated")] to: String,
        #[graphql(desc = "Your message to include above forwarded content")] body: Option<String>,
        #[graphql(desc = "CC recipients, comma-separated")] cc: Option<String>,
        #[graphql(desc = "BCC recipients, comma-separated")] bcc: Option<String>,
        #[graphql(desc = "Send from a specific identity/email address")] from: Option<String>,
        #[graphql(desc = "Token from PREVIEW response — required for CONFIRM/DRAFT")]
        confirmation_token: Option<String>,
    ) -> Result<GqlComposeResult> {
        let body_str = body.as_deref().unwrap_or("");
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&email_id, &to, body_str]);
        let client = app_ctx.require_jmap()?;
        let mut client = client.lock().await;

        let original = client.get_email(&email_id).await?;
        let to_addrs = parse_addresses(&to);
        let cc_addrs = cc.as_deref().map(parse_addresses).unwrap_or_default();
        let bcc_addrs = bcc.as_deref().map(parse_addresses).unwrap_or_default();

        let subject = if original
            .subject
            .as_ref()
            .is_some_and(|s| s.to_lowercase().starts_with("fwd:"))
        {
            original.subject.clone().unwrap_or_default()
        } else {
            format!("Fwd: {}", original.subject.as_deref().unwrap_or(""))
        };

        if matches!(action, SendAction::Preview) {
            let original_body = original.text_content().unwrap_or("");
            let sender = format_addrs(&original.from.clone().unwrap_or_default());

            return Ok(GqlComposeResult {
                success: true,
                email_id: None,
                preview: Some(format!(
                    "FORWARD PREVIEW:\nTo: {}\nCC: {}\nBCC: {}\nSubject: {}\nForwarding from: {}\n\n--- Your Message ---\n{}\n\n--- Forwarded ---\nFrom: {}\nDate: {}\nSubject: {}\n\n{}",
                    format_addrs(&to_addrs),
                    if cc_addrs.is_empty() {
                        "(none)".to_string()
                    } else {
                        format_addrs(&cc_addrs)
                    },
                    if bcc_addrs.is_empty() {
                        "(none)".to_string()
                    } else {
                        format_addrs(&bcc_addrs)
                    },
                    subject,
                    sender,
                    body_str,
                    sender,
                    original.received_at.as_deref().unwrap_or("unknown"),
                    original.subject.as_deref().unwrap_or(""),
                    original_body,
                )),
                confirmation_token: Some(token),
                error: None,
            });
        }

        if confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        let draft = matches!(action, SendAction::Draft);
        match client
            .forward_email(
                &original,
                to_addrs,
                body_str,
                crate::jmap::ComposeParams {
                    cc: cc_addrs,
                    bcc: bcc_addrs,
                    from: from.as_deref(),
                    draft,
                },
            )
            .await
        {
            Ok(eid) => Ok(GqlComposeResult {
                success: true,
                email_id: Some(eid),
                preview: None,
                confirmation_token: None,
                error: None,
            }),
            Err(e) => Ok(GqlComposeResult {
                success: false,
                email_id: None,
                preview: None,
                confirmation_token: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Move an email to a different mailbox/folder.
    async fn move_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID to move")] email_id: String,
        #[graphql(desc = "Target mailbox name (e.g., 'Archive', 'Trash') or role")]
        target_mailbox: String,
    ) -> Result<GqlStatus> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let mut client = client.lock().await;

        let email = client.get_email(&email_id).await?;
        let target = client.find_mailbox(&target_mailbox).await?;

        match client.move_email(&email_id, &target.id).await {
            Ok(()) => Ok(GqlStatus {
                success: true,
                message: Some(format!(
                    "Moved \"{}\" to {}",
                    email.subject.as_deref().unwrap_or("(no subject)"),
                    target.name
                )),
                error: None,
            }),
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Mark an email as read or unread.
    async fn mark_as_read(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID")] email_id: String,
        #[graphql(desc = "true to mark read, false to mark unread (default: true)")] read: Option<
            bool,
        >,
    ) -> Result<GqlStatus> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;
        let read = read.unwrap_or(true);

        let email = client.get_email(&email_id).await?;
        let mut keywords = email.keywords.clone();
        if read {
            keywords.insert("$seen".to_string(), true);
        } else {
            keywords.remove("$seen");
        }

        match client.set_keywords(&email_id, keywords).await {
            Ok(()) => {
                let status = if read { "read" } else { "unread" };
                Ok(GqlStatus {
                    success: true,
                    message: Some(format!(
                        "Marked \"{}\" as {status}",
                        email.subject.as_deref().unwrap_or("(no subject)")
                    )),
                    error: None,
                })
            }
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Mark an email as spam. Moves to Junk AND trains the spam filter.
    /// Destructive — requires PREVIEW + CONFIRM with confirmation_token (SEC-08).
    async fn mark_as_spam(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID")] email_id: String,
        #[graphql(desc = "PREVIEW first, then CONFIRM")] action: SpamAction,
        #[graphql(desc = "Confirmation token returned by PREVIEW")] confirmation_token: Option<
            String,
        >,
    ) -> Result<GqlStatus> {
        let app_ctx = ctx.data::<super::AppContext>()?;
        let token = app_ctx.confirmation_token(&[&email_id]);

        // Validate token BEFORE acquiring JMAP lock (defense-in-depth: malicious callers
        // cannot force JMAP acquisition without a valid token).
        if matches!(action, SpamAction::Confirm) && confirmation_token.as_deref() != Some(&token) {
            return Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(
                    "Missing or invalid confirmation_token. Use action=PREVIEW first.".to_string(),
                ),
            });
        }

        let client = app_ctx.require_jmap()?;
        let mut client = client.lock().await;
        let email = client.get_email(&email_id).await?;

        if matches!(action, SpamAction::Preview) {
            let sender = email
                .from
                .as_ref()
                .and_then(|f| f.first())
                .map(|a| a.to_string())
                .unwrap_or_else(|| "(unknown)".to_string());
            return Ok(GqlStatus {
                success: true,
                message: Some(format!(
                    "SPAM PREVIEW — This will:\n1. Move to Junk folder\n2. Train spam filter\n\nEmail: \"{}\"\nFrom: {}\n\nUse action=CONFIRM with confirmation_token=\"{}\" to proceed.\n\nConfirmation token: {}",
                    email.subject.as_deref().unwrap_or("(no subject)"),
                    sender,
                    token,
                    token
                )),
                error: None,
            });
        }

        // CONFIRM branch — token already validated above.
        match client.mark_spam(&email_id).await {
            Ok(()) => Ok(GqlStatus {
                success: true,
                message: Some(format!(
                    "Marked as spam: \"{}\"",
                    email.subject.as_deref().unwrap_or("(no subject)")
                )),
                error: None,
            }),
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create a new masked email address.
    async fn create_masked_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The website/domain this masked email is for")] for_domain: Option<String>,
        #[graphql(desc = "A note to remember what this is for")] description: Option<String>,
        #[graphql(desc = "Custom prefix for the email address")] prefix: Option<String>,
    ) -> Result<GqlMaskedEmail> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;
        let masked = client
            .create_masked_email(
                for_domain.as_deref(),
                description.as_deref(),
                prefix.as_deref(),
            )
            .await?;
        Ok(GqlMaskedEmail::from(masked))
    }

    /// Enable a disabled masked email address.
    async fn enable_masked_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The masked email ID")] id: String,
    ) -> Result<GqlStatus> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;
        match client
            .update_masked_email(&id, Some("enabled"), None, None)
            .await
        {
            Ok(()) => Ok(GqlStatus {
                success: true,
                message: Some(format!("Masked email {id} enabled.")),
                error: None,
            }),
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Disable a masked email address. Emails sent to it will be rejected.
    async fn disable_masked_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The masked email ID")] id: String,
    ) -> Result<GqlStatus> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;
        match client
            .update_masked_email(&id, Some("disabled"), None, None)
            .await
        {
            Ok(()) => Ok(GqlStatus {
                success: true,
                message: Some(format!("Masked email {id} disabled.")),
                error: None,
            }),
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Permanently delete a masked email address. Cannot be undone!
    async fn delete_masked_email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The masked email ID")] id: String,
    ) -> Result<GqlStatus> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;
        match client
            .update_masked_email(&id, Some("deleted"), None, None)
            .await
        {
            Ok(()) => Ok(GqlStatus {
                success: true,
                message: Some(format!("Masked email {id} deleted.")),
                error: None,
            }),
            Err(e) => Ok(GqlStatus {
                success: false,
                message: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

// ============ Formatting helpers (preview only) ============

fn format_addrs(addrs: &[EmailAddress]) -> String {
    if addrs.is_empty() {
        "(none)".to_string()
    } else {
        addrs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_send_preview(
    to: &[EmailAddress],
    cc: &[EmailAddress],
    bcc: &[EmailAddress],
    subject: &str,
    body: &str,
) -> String {
    format!(
        "EMAIL PREVIEW:\nTo: {}\nCC: {}\nBCC: {}\nSubject: {}\n\n--- Body ---\n{}\n\nTo send: use action=CONFIRM. To save draft: use action=DRAFT.",
        format_addrs(to),
        if cc.is_empty() {
            "(none)".to_string()
        } else {
            format_addrs(cc)
        },
        if bcc.is_empty() {
            "(none)".to_string()
        } else {
            format_addrs(bcc)
        },
        subject,
        body
    )
}

#[cfg(test)]
mod tests {
    use super::super::{AppContext, build_schema};

    fn test_schema() -> super::super::FastmailSchema {
        build_schema(AppContext::new_with_key(None, [0u8; 32]))
    }

    /// CONFIRM without any token returns "Missing or invalid confirmation_token" error.
    #[tokio::test]
    async fn mark_as_spam_confirm_rejects_missing_token() {
        let schema = test_schema();
        let result = schema
            .execute(
                r#"mutation {
                    markAsSpam(emailId: "email-test-id", action: CONFIRM) {
                        success
                        error
                    }
                }"#,
            )
            .await;
        // Either a GraphQL-level error (auth) or a business-level success:false with the
        // "Missing or invalid confirmation_token" message.  Since there is no JMAP client
        // and the token check happens BEFORE require_jmap, we must see the token error.
        let data = result.data.into_json().unwrap();
        let success = data["markAsSpam"]["success"].as_bool().unwrap_or(true);
        let error = data["markAsSpam"]["error"].as_str().unwrap_or("");
        assert!(!success, "Expected success=false, got: {data:?}");
        assert!(
            error.contains("Missing or invalid confirmation_token"),
            "Expected token error, got: {error}"
        );
    }

    /// CONFIRM with a wrong token returns the same rejection error.
    #[tokio::test]
    async fn mark_as_spam_confirm_rejects_wrong_token() {
        let schema = test_schema();
        let result = schema
            .execute(
                r#"mutation {
                    markAsSpam(emailId: "email-test-id", action: CONFIRM, confirmationToken: "totally-wrong") {
                        success
                        error
                    }
                }"#,
            )
            .await;
        let data = result.data.into_json().unwrap();
        let success = data["markAsSpam"]["success"].as_bool().unwrap_or(true);
        let error = data["markAsSpam"]["error"].as_str().unwrap_or("");
        assert!(!success, "Expected success=false, got: {data:?}");
        assert!(
            error.contains("Missing or invalid confirmation_token"),
            "Expected token error, got: {error}"
        );
    }

    /// Different AppContext instances produce different tokens for the same email_id.
    #[test]
    fn mark_as_spam_different_keys_produce_different_tokens() {
        let ctx_a = AppContext::new_with_key(None, [1u8; 32]);
        let ctx_b = AppContext::new_with_key(None, [2u8; 32]);
        let token_a = ctx_a.confirmation_token(&["email-test-id"]);
        let token_b = ctx_b.confirmation_token(&["email-test-id"]);
        assert_ne!(
            token_a, token_b,
            "Different keys must produce different tokens"
        );
    }
}
