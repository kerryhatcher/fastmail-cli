//! GraphQL query resolvers

use async_graphql::{Context, Object, Result};

use super::types::*;

pub struct QueryRoot;

fn require_jmap_client<'a>(
    ctx: &'a Context<'a>,
) -> Result<std::sync::Arc<tokio::sync::Mutex<crate::jmap::JmapClient>>> {
    ctx.data::<super::JmapContext>()?
        .client
        .clone()
        .ok_or_else(|| {
            async_graphql::Error::new(
                "JMAP token not configured. Mail operations require FASTMAIL_API_TOKEN.",
            )
        })
}

#[Object]
#[allow(clippy::too_many_arguments)]
impl QueryRoot {
    /// List all mailboxes (folders) with unread counts. Start here to discover available folders.
    async fn mailboxes(&self, ctx: &Context<'_>) -> Result<Vec<GqlMailbox>> {
        let client = require_jmap_client(ctx)?;
        let mut client = client.lock().await;
        let mut mailboxes = client.list_mailboxes().await?;
        mailboxes.sort_by(|a, b| match (&a.role, &b.role) {
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        Ok(mailboxes.into_iter().map(GqlMailbox::from).collect())
    }

    /// List emails in a specific mailbox/folder.
    async fn emails(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "Mailbox name (e.g., 'INBOX', 'Sent') or role (e.g., 'inbox', 'sent', 'drafts')"
        )]
        mailbox: String,
        #[graphql(desc = "Maximum number of emails to return (default 25, max 100)")] limit: Option<
            u32,
        >,
    ) -> Result<Vec<GqlEmailSummary>> {
        let client = require_jmap_client(ctx)?;
        let mut client = client.lock().await;
        let limit = limit.unwrap_or(25).min(100);
        let mb = client.find_mailbox(&mailbox).await?;
        let emails = client.list_emails(&mb.id, limit).await?;
        Ok(emails.into_iter().map(Into::into).collect())
    }

    /// Get full content of a specific email by ID. Includes nested attachments —
    /// select `attachments { content { ... } }` to download attachment data in the same query.
    async fn email(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID (from emails or searchEmails queries)")] id: String,
    ) -> Result<Option<GqlEmail>> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        match client.get_email(&id).await {
            Ok(email) => Ok(Some(GqlEmail(email))),
            Err(crate::error::Error::EmailNotFound(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all emails in a thread/conversation with full content. Returns emails sorted
    /// oldest-first. Each email has full body and nested attachment access.
    async fn thread(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Any email ID in the thread")] email_id: String,
    ) -> Result<GqlThread> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        let mut emails = client.get_thread(&email_id).await?;
        emails.sort_by(|a, b| a.received_at.cmp(&b.received_at));
        let total = emails.len();
        Ok(GqlThread { emails, total })
    }

    /// Search emails with flexible filters.
    async fn search_emails(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "General search — searches subject, body, from, and to fields")]
        query: Option<String>,
        #[graphql(desc = "Search sender address/name")] from: Option<String>,
        #[graphql(desc = "Search recipient address/name")] to: Option<String>,
        #[graphql(desc = "Search CC recipients")] cc: Option<String>,
        #[graphql(desc = "Search subject line only")] subject: Option<String>,
        #[graphql(desc = "Search email body only")] body: Option<String>,
        #[graphql(desc = "Limit search to a specific mailbox/folder")] mailbox: Option<String>,
        #[graphql(desc = "Only emails with attachments")] has_attachment: Option<bool>,
        #[graphql(desc = "Emails before this date (YYYY-MM-DD or ISO 8601)")] before: Option<
            String,
        >,
        #[graphql(desc = "Emails after this date (YYYY-MM-DD or ISO 8601)")] after: Option<String>,
        #[graphql(desc = "Only unread emails")] unread: Option<bool>,
        #[graphql(desc = "Only flagged/starred emails")] flagged: Option<bool>,
        #[graphql(desc = "Maximum number of results (default 25, max 100)")] limit: Option<u32>,
    ) -> Result<Vec<GqlEmailSummary>> {
        let client = require_jmap_client(ctx)?;
        let mut client = client.lock().await;
        let limit = limit.unwrap_or(25).min(100);

        let filter = crate::commands::SearchFilter {
            text: query,
            from,
            to,
            cc,
            bcc: None,
            subject,
            body,
            mailbox: None,
            has_attachment: has_attachment.unwrap_or(false),
            min_size: None,
            max_size: None,
            before,
            after,
            unread: unread.unwrap_or(false),
            flagged: flagged.unwrap_or(false),
        };

        let mailbox_id = if let Some(ref name) = mailbox {
            client.find_mailbox(name).await.ok().map(|m| m.id)
        } else {
            None
        };

        let emails = client
            .search_emails_filtered(&filter, mailbox_id.as_deref(), limit)
            .await?;
        Ok(emails.into_iter().map(Into::into).collect())
    }

    /// List attachment metadata for an email. Select `content` on each attachment to fetch data.
    async fn attachments(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID")] email_id: String,
    ) -> Result<Vec<GqlAttachment>> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        let email = client.get_email(&email_id).await?;
        Ok(GqlEmail(email).make_attachments())
    }

    /// Get a single attachment by blob ID. Select `content` to fetch its data.
    async fn attachment(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The email ID the attachment belongs to")] email_id: String,
        #[graphql(desc = "The blob ID of the attachment (from attachments query)")] blob_id: String,
    ) -> Result<Option<GqlAttachment>> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        let email = client.get_email(&email_id).await?;
        Ok(GqlEmail(email)
            .make_attachments()
            .into_iter()
            .find(|a| a.blob_id == blob_id))
    }

    /// List all sender identities on the account. Includes signatures and default reply-to/bcc.
    async fn identities(&self, ctx: &Context<'_>) -> Result<Vec<GqlIdentity>> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        let identities = client.list_identities().await?;
        Ok(identities.into_iter().map(GqlIdentity::from).collect())
    }

    /// List all masked email addresses.
    async fn masked_emails(&self, ctx: &Context<'_>) -> Result<Vec<GqlMaskedEmail>> {
        let client = require_jmap_client(ctx)?;
        let client = client.lock().await;
        let mut masked = client.list_masked_emails().await?;
        masked.sort_by(|a, b| {
            let a_enabled = a.state.as_deref() == Some("enabled");
            let b_enabled = b.state.as_deref() == Some("enabled");
            match (a_enabled, b_enabled) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.email.cmp(&b.email),
            }
        });
        Ok(masked.into_iter().map(GqlMaskedEmail::from).collect())
    }

    /// Search contacts by name, email, or organization. Requires FASTMAIL_APP_PASSWORD.
    async fn contacts(
        &self,
        #[graphql(desc = "Search query — matches name, email, or organization")] query: String,
    ) -> Result<Vec<GqlContact>> {
        let config = crate::config::Config::load()?;
        let username = config.get_username().map_err(|_| {
            async_graphql::Error::new("Username not configured. Set FASTMAIL_USERNAME env var.")
        })?;
        let app_password = config.get_app_password().map_err(|_| {
            async_graphql::Error::new(
                "App password not configured. Set FASTMAIL_APP_PASSWORD env var (API tokens don't work for CardDAV).",
            )
        })?;

        let client = crate::carddav::CardDavClient::new(username, app_password)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let contacts = client.search_contacts(&query).await?;
        Ok(contacts.into_iter().map(GqlContact::from).collect())
    }

    /// List calendars available through Fastmail CalDAV. Requires FASTMAIL_APP_PASSWORD.
    async fn calendars(&self) -> Result<Vec<GqlCalendar>> {
        let config = crate::config::Config::load()?;
        let username = config.get_username().map_err(|_| {
            async_graphql::Error::new("Username not configured. Set FASTMAIL_USERNAME env var.")
        })?;
        let app_password = config.get_app_password().map_err(|_| {
            async_graphql::Error::new(
                "App password not configured. Set FASTMAIL_APP_PASSWORD env var (API tokens don't work for CalDAV).",
            )
        })?;

        let client = crate::caldav::CalDavClient::new(username, app_password)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let calendars = client.list_calendars().await?;
        Ok(calendars.into_iter().map(Into::into).collect())
    }

    /// List events. Defaults to future events for the rest of today when no explicit range is supplied.
    /// Use `week: true` for the current week, or provide both `start` and `end` for an explicit range.
    async fn events(
        &self,
        #[graphql(desc = "Optional calendar ID to scope the query")] calendar_id: Option<String>,
        #[graphql(desc = "Explicit range start (YYYY-MM-DD or RFC3339). Requires end.")]
        start: Option<String>,
        #[graphql(desc = "Explicit range end (YYYY-MM-DD or RFC3339). Requires start.")]
        end: Option<String>,
        #[graphql(desc = "Use the current week range instead of the default today range")]
        week: Option<bool>,
    ) -> Result<Vec<GqlCalendarEvent>> {
        let events = crate::commands::list_events_record(
            calendar_id.as_deref(),
            start.as_deref(),
            end.as_deref(),
            week.unwrap_or(false),
        )
        .await?;
        Ok(events.into_iter().map(Into::into).collect())
    }

    /// Fetch one event by UID. Returns null when the event does not exist.
    async fn event(
        &self,
        #[graphql(desc = "Event UID")] id: String,
        #[graphql(desc = "Optional calendar ID hint")] calendar_id: Option<String>,
    ) -> Result<Option<GqlCalendarEvent>> {
        match crate::commands::get_event_record(&id, calendar_id.as_deref()).await {
            Ok(event) => Ok(Some(event.into())),
            Err(crate::error::Error::EventNotFound(_)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}
