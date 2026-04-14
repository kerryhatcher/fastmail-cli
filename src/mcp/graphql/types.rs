//! GraphQL type wrappers around existing model structs

use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

use crate::caldav::{
    Calendar, CalendarEvent, EventAttendee, EventDateTime, EventRecurrence, EventReminder,
};
use crate::carddav::{Contact, ContactEmail, ContactGroup, ContactPhone};
use crate::models::{Email, EmailAddress, Identity, Mailbox, MaskedEmail};

// ============ Output Types ============

#[derive(SimpleObject)]
#[graphql(name = "Mailbox")]
pub struct GqlMailbox {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub role: Option<String>,
    pub total_emails: u32,
    pub unread_emails: u32,
    pub total_threads: u32,
    pub unread_threads: u32,
    pub sort_order: u32,
}

impl From<Mailbox> for GqlMailbox {
    fn from(m: Mailbox) -> Self {
        Self {
            id: m.id,
            name: m.name,
            parent_id: m.parent_id,
            role: m.role,
            total_emails: m.total_emails,
            unread_emails: m.unread_emails,
            total_threads: m.total_threads,
            unread_threads: m.unread_threads,
            sort_order: m.sort_order,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "EmailAddress")]
pub struct GqlEmailAddress {
    pub name: Option<String>,
    pub email: String,
}

impl From<EmailAddress> for GqlEmailAddress {
    fn from(a: EmailAddress) -> Self {
        Self {
            name: a.name,
            email: a.email,
        }
    }
}

pub(crate) fn convert_addrs(addrs: Option<Vec<EmailAddress>>) -> Vec<GqlEmailAddress> {
    addrs
        .unwrap_or_default()
        .into_iter()
        .map(GqlEmailAddress::from)
        .collect()
}

/// Compact email summary returned by list/search queries
#[derive(SimpleObject)]
#[graphql(name = "EmailSummary")]
pub struct GqlEmailSummary {
    pub id: String,
    pub thread_id: Option<String>,
    pub subject: Option<String>,
    #[graphql(name = "from")]
    pub sender: Vec<GqlEmailAddress>,
    #[graphql(name = "to")]
    pub recipients: Vec<GqlEmailAddress>,
    #[graphql(name = "cc")]
    pub cc_recipients: Vec<GqlEmailAddress>,
    pub received_at: Option<String>,
    pub preview: Option<String>,
    pub has_attachment: bool,
    pub is_unread: bool,
    pub is_flagged: bool,
    pub size: u64,
}

impl From<Email> for GqlEmailSummary {
    fn from(e: Email) -> Self {
        let is_unread = e.is_unread();
        let is_flagged = e.is_flagged();
        Self {
            id: e.id,
            thread_id: e.thread_id,
            subject: e.subject,
            sender: convert_addrs(e.from),
            recipients: convert_addrs(e.to),
            cc_recipients: convert_addrs(e.cc),
            received_at: e.received_at,
            preview: e.preview,
            has_attachment: e.has_attachment,
            is_unread,
            is_flagged,
            size: e.size,
        }
    }
}

/// Full email with body content and nested attachment resolution.
/// Address fields are precomputed once at construction and shared via Arc to avoid
/// per-resolver-call conversion and cloning.
pub struct GqlEmail {
    pub inner: Email,
    from: std::sync::Arc<Vec<GqlEmailAddress>>,
    to: std::sync::Arc<Vec<GqlEmailAddress>>,
    cc: std::sync::Arc<Vec<GqlEmailAddress>>,
    bcc: std::sync::Arc<Vec<GqlEmailAddress>>,
    reply_to: std::sync::Arc<Vec<GqlEmailAddress>>,
}

impl GqlEmail {
    /// Construct a GqlEmail, precomputing all address fields once.
    pub fn new(email: Email) -> Self {
        let from = std::sync::Arc::new(convert_addrs(email.from.clone()));
        let to = std::sync::Arc::new(convert_addrs(email.to.clone()));
        let cc = std::sync::Arc::new(convert_addrs(email.cc.clone()));
        let bcc = std::sync::Arc::new(convert_addrs(email.bcc.clone()));
        let reply_to = std::sync::Arc::new(convert_addrs(email.reply_to.clone()));
        Self {
            inner: email,
            from,
            to,
            cc,
            bcc,
            reply_to,
        }
    }

    /// Build attachment list from the inner email — shared by the nested resolver
    /// and the top-level `attachments`/`attachment` queries.
    pub fn make_attachments(&self) -> Vec<GqlAttachment> {
        self.inner
            .attachments
            .as_ref()
            .map(|atts| {
                atts.iter()
                    .filter(|a| a.blob_id.is_some())
                    .map(|a| GqlAttachment {
                        blob_id: a.blob_id.clone().unwrap_or_default(),
                        name: a.name.clone(),
                        content_type: a.content_type.clone(),
                        size: a.size,
                        disposition: a.disposition.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[Object(name = "Email")]
impl GqlEmail {
    async fn id(&self) -> &str {
        &self.inner.id
    }
    async fn blob_id(&self) -> Option<&str> {
        self.inner.blob_id.as_deref()
    }
    async fn thread_id(&self) -> Option<&str> {
        self.inner.thread_id.as_deref()
    }
    async fn subject(&self) -> Option<&str> {
        self.inner.subject.as_deref()
    }
    async fn from(&self) -> &[GqlEmailAddress] {
        &self.from
    }
    async fn to(&self) -> &[GqlEmailAddress] {
        &self.to
    }
    async fn cc(&self) -> &[GqlEmailAddress] {
        &self.cc
    }
    async fn bcc(&self) -> &[GqlEmailAddress] {
        &self.bcc
    }
    async fn reply_to(&self) -> &[GqlEmailAddress] {
        &self.reply_to
    }
    async fn received_at(&self) -> Option<&str> {
        self.inner.received_at.as_deref()
    }
    async fn sent_at(&self) -> Option<&str> {
        self.inner.sent_at.as_deref()
    }
    async fn preview(&self) -> Option<&str> {
        self.inner.preview.as_deref()
    }
    async fn has_attachment(&self) -> bool {
        self.inner.has_attachment
    }
    async fn is_unread(&self) -> bool {
        self.inner.is_unread()
    }
    async fn is_flagged(&self) -> bool {
        self.inner.is_flagged()
    }
    async fn is_draft(&self) -> bool {
        self.inner.is_draft()
    }
    async fn size(&self) -> u64 {
        self.inner.size
    }
    async fn message_id(&self) -> Option<&Vec<String>> {
        self.inner.message_id.as_ref()
    }
    async fn in_reply_to(&self) -> Option<&Vec<String>> {
        self.inner.in_reply_to.as_ref()
    }
    async fn references(&self) -> Option<&Vec<String>> {
        self.inner.references.as_ref()
    }
    /// Plain text body content
    async fn text_body(&self) -> Option<&str> {
        self.inner.text_content()
    }
    /// HTML body content
    async fn html_body(&self) -> Option<&str> {
        self.inner.html_content()
    }
    /// Attachments with metadata. Select `content` on an attachment to fetch its data (images
    /// are base64-encoded, documents have text extracted). Content is lazily resolved — only
    /// fetched when you include it in your query.
    async fn attachments(&self) -> Vec<GqlAttachment> {
        self.make_attachments()
    }
    /// Mailbox IDs this email belongs to
    async fn mailbox_ids(&self) -> Vec<String> {
        self.inner.mailbox_ids.keys().cloned().collect()
    }

    /// Keywords (flags) on this email: $seen, $flagged, $draft, $junk, etc.
    async fn keywords(&self) -> Vec<String> {
        self.inner.keywords.keys().cloned().collect()
    }
}

/// Attachment with metadata and lazy content resolution.
/// Query just metadata fields (blobId, name, size, etc.) for listings,
/// or include `content` to download and process the attachment data.
pub struct GqlAttachment {
    pub blob_id: String,
    pub name: Option<String>,
    pub content_type: Option<String>,
    pub size: u64,
    pub disposition: Option<String>,
}

#[Object(name = "Attachment")]
impl GqlAttachment {
    async fn blob_id(&self) -> &str {
        &self.blob_id
    }
    async fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    async fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
    async fn size(&self) -> u64 {
        self.size
    }
    async fn disposition(&self) -> Option<&str> {
        self.disposition.as_deref()
    }
    /// Fetch the actual attachment content. Images are resized and base64-encoded,
    /// documents have text extracted. Only fetched when this field is included in the query.
    async fn content(&self, ctx: &Context<'_>) -> Result<GqlAttachmentContent> {
        let client = ctx.data::<super::AppContext>()?.require_jmap()?;
        let client = client.lock().await;

        let content_type = self
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let name = self.name.as_deref().unwrap_or("attachment");

        let data = client.download_blob(&self.blob_id).await?;

        let mime = if crate::util::is_image(content_type, name) {
            crate::util::infer_image_mime(name).unwrap_or(content_type)
        } else {
            content_type
        };

        // Images — resize and base64 encode
        if crate::util::is_image(mime, name) {
            return match crate::util::resize_image(data.as_ref(), mime, crate::util::MCP_IMAGE_MAX_BYTES) {
                Ok((processed_data, _mime_type)) => {
                    let base64_data = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &processed_data,
                    );
                    Ok(GqlAttachmentContent {
                        size: processed_data.len(),
                        base64_content: Some(base64_data),
                        text_content: None,
                        info: None,
                    })
                }
                Err(e) => Err(async_graphql::Error::new(format!(
                    "Failed to process image: {e}"
                ))),
            };
        }

        // Documents — extract text
        match crate::util::extract_text(data.as_ref(), name).await {
            Ok(Some(text)) => {
                return Ok(GqlAttachmentContent {
                    size: data.len(),
                    base64_content: None,
                    text_content: Some(text),
                    info: None,
                });
            }
            Ok(None) => {}
            Err(e) => {
                return Err(async_graphql::Error::new(format!(
                    "Failed to extract text: {e}"
                )));
            }
        }

        // Binary fallback
        Ok(GqlAttachmentContent {
            size: data.len(),
            base64_content: None,
            text_content: None,
            info: Some("Binary attachment — cannot be displayed directly.".to_string()),
        })
    }
}

#[derive(SimpleObject)]
#[graphql(name = "AttachmentContent")]
pub struct GqlAttachmentContent {
    pub size: usize,
    /// For images: base64-encoded image data
    pub base64_content: Option<String>,
    /// For documents: extracted text content
    pub text_content: Option<String>,
    /// Description when content can't be returned directly
    pub info: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "Identity")]
pub struct GqlIdentity {
    pub id: String,
    pub name: String,
    pub email: String,
    pub may_delete: bool,
    pub text_signature: Option<String>,
    pub html_signature: Option<String>,
    pub reply_to: Vec<GqlEmailAddress>,
    pub bcc: Vec<GqlEmailAddress>,
}

impl From<Identity> for GqlIdentity {
    fn from(i: Identity) -> Self {
        Self {
            id: i.id,
            name: i.name,
            email: i.email,
            may_delete: i.may_delete,
            text_signature: i.text_signature,
            html_signature: i.html_signature,
            reply_to: convert_addrs(i.reply_to),
            bcc: convert_addrs(i.bcc),
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "MaskedEmail")]
pub struct GqlMaskedEmail {
    pub id: String,
    pub email: String,
    pub state: Option<String>,
    pub for_domain: Option<String>,
    pub description: Option<String>,
    pub last_message_at: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
    pub url: Option<String>,
}

impl From<MaskedEmail> for GqlMaskedEmail {
    fn from(m: MaskedEmail) -> Self {
        Self {
            id: m.id,
            email: m.email,
            state: m.state,
            for_domain: m.for_domain,
            description: m.description,
            last_message_at: m.last_message_at,
            created_at: m.created_at,
            created_by: m.created_by,
            url: m.url,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "ContactEmail")]
pub struct GqlContactEmail {
    pub email: String,
    pub label: Option<String>,
}

impl From<ContactEmail> for GqlContactEmail {
    fn from(e: ContactEmail) -> Self {
        Self {
            email: e.email,
            label: e.label,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "ContactPhone")]
pub struct GqlContactPhone {
    pub number: String,
    pub label: Option<String>,
}

impl From<ContactPhone> for GqlContactPhone {
    fn from(p: ContactPhone) -> Self {
        Self {
            number: p.number,
            label: p.label,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "Contact")]
pub struct GqlContact {
    pub id: String,
    pub name: String,
    pub emails: Vec<GqlContactEmail>,
    pub phones: Vec<GqlContactPhone>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub address: Option<String>,
    pub href: Option<String>,
    pub etag: Option<String>,
}

impl From<Contact> for GqlContact {
    fn from(c: Contact) -> Self {
        Self {
            id: c.id,
            name: c.name,
            emails: c.emails.into_iter().map(GqlContactEmail::from).collect(),
            phones: c.phones.into_iter().map(GqlContactPhone::from).collect(),
            organization: c.organization,
            title: c.title,
            notes: c.notes,
            address: c.address,
            href: c.href,
            etag: c.etag,
        }
    }
}

/// Contact group (vCard KIND:group / X-ADDRESSBOOKSERVER-KIND:group)
#[derive(SimpleObject)]
#[graphql(name = "ContactGroup")]
pub struct GqlContactGroup {
    /// Group unique identifier (vCard UID)
    pub id: String,
    /// Group display name (vCard FN)
    pub name: String,
    /// Number of members in the group
    pub member_count: i32,
    /// Resolved member contacts (populated by get_group, empty in list_groups)
    pub members: Vec<GqlContact>,
    /// Server-assigned resource URL
    pub href: Option<String>,
    /// HTTP ETag for concurrency control
    pub etag: Option<String>,
}

impl From<ContactGroup> for GqlContactGroup {
    fn from(g: ContactGroup) -> Self {
        Self {
            member_count: g.member_uids.len() as i32,
            id: g.id,
            name: g.name,
            members: vec![],
            href: g.href,
            etag: g.etag,
        }
    }
}

impl GqlContactGroup {
    /// Construct a GqlContactGroup with resolved member contacts.
    pub fn with_members(group: ContactGroup, contacts: Vec<Contact>) -> Self {
        let member_count = group.member_uids.len() as i32;
        Self {
            id: group.id,
            name: group.name,
            member_count,
            members: contacts.into_iter().map(GqlContact::from).collect(),
            href: group.href,
            etag: group.etag,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "Calendar")]
pub struct GqlCalendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub href: String,
    pub etag: Option<String>,
    pub ctag: Option<String>,
    pub is_default: bool,
}

impl From<Calendar> for GqlCalendar {
    fn from(calendar: Calendar) -> Self {
        Self {
            id: calendar.id,
            name: calendar.name,
            color: calendar.color,
            description: calendar.description,
            href: calendar.href,
            etag: calendar.etag,
            ctag: calendar.ctag,
            is_default: calendar.is_default,
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDateTime")]
pub struct GqlEventDateTime {
    pub value: String,
    pub timezone: Option<String>,
    pub all_day: bool,
}

impl From<EventDateTime> for GqlEventDateTime {
    fn from(value: EventDateTime) -> Self {
        Self {
            value: value.value,
            timezone: value.timezone,
            all_day: value.all_day,
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventAttendee")]
pub struct GqlEventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub partstat: Option<String>,
    pub rsvp: Option<bool>,
}

impl From<EventAttendee> for GqlEventAttendee {
    fn from(attendee: EventAttendee) -> Self {
        Self {
            email: attendee.email,
            name: attendee.name,
            role: attendee.role,
            partstat: attendee.partstat,
            rsvp: attendee.rsvp,
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventRecurrence")]
pub struct GqlEventRecurrence {
    pub frequency: String,
    pub interval: Option<u32>,
    pub count: Option<u32>,
    pub until: Option<String>,
    pub by_day: Vec<String>,
}

impl From<EventRecurrence> for GqlEventRecurrence {
    fn from(recurrence: EventRecurrence) -> Self {
        Self {
            frequency: recurrence.frequency,
            interval: recurrence.interval,
            count: recurrence.count,
            until: recurrence.until,
            by_day: recurrence.by_day,
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "EventReminder")]
pub struct GqlEventReminder {
    pub minutes_before: i32,
    pub action: Option<String>,
}

impl From<EventReminder> for GqlEventReminder {
    fn from(reminder: EventReminder) -> Self {
        Self {
            minutes_before: reminder.minutes_before,
            action: reminder.action,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "CalendarEvent")]
pub struct GqlCalendarEvent {
    pub id: String,
    pub calendar_id: String,
    pub calendar_name: Option<String>,
    pub href: Option<String>,
    pub etag: Option<String>,
    pub title: String,
    pub start: GqlEventDateTime,
    pub end: GqlEventDateTime,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Vec<GqlEventAttendee>,
    pub recurrence: Option<GqlEventRecurrence>,
    pub reminders: Vec<GqlEventReminder>,
}

impl From<CalendarEvent> for GqlCalendarEvent {
    fn from(event: CalendarEvent) -> Self {
        Self {
            id: event.id,
            calendar_id: event.calendar_id,
            calendar_name: event.calendar_name,
            href: event.href,
            etag: event.etag,
            title: event.title,
            start: event.start.into(),
            end: event.end.into(),
            location: event.location,
            description: event.description,
            attendees: event.attendees.into_iter().map(Into::into).collect(),
            recurrence: event.recurrence.map(Into::into),
            reminders: event.reminders.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(InputObject, Clone)]
#[graphql(name = "EventAttendeeInput")]
pub struct GqlEventAttendeeInput {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub partstat: Option<String>,
    pub rsvp: Option<bool>,
}

impl From<GqlEventAttendeeInput> for EventAttendee {
    fn from(attendee: GqlEventAttendeeInput) -> Self {
        Self {
            email: attendee.email,
            name: attendee.name,
            role: attendee.role,
            partstat: attendee.partstat,
            rsvp: attendee.rsvp,
        }
    }
}

#[derive(InputObject, Clone)]
#[graphql(name = "EventRecurrenceInput")]
pub struct GqlEventRecurrenceInput {
    pub frequency: String,
    pub interval: Option<u32>,
    pub count: Option<u32>,
    pub until: Option<String>,
    pub by_day: Option<Vec<String>>,
}

impl From<GqlEventRecurrenceInput> for EventRecurrence {
    fn from(recurrence: GqlEventRecurrenceInput) -> Self {
        Self {
            frequency: recurrence.frequency,
            interval: recurrence.interval,
            count: recurrence.count,
            until: recurrence.until,
            by_day: recurrence.by_day.unwrap_or_default(),
        }
    }
}

#[derive(InputObject, Clone)]
#[graphql(name = "EventReminderInput")]
pub struct GqlEventReminderInput {
    pub minutes_before: i32,
    pub action: Option<String>,
}

impl From<GqlEventReminderInput> for EventReminder {
    fn from(reminder: GqlEventReminderInput) -> Self {
        Self {
            minutes_before: reminder.minutes_before,
            action: reminder.action,
        }
    }
}

// ============ Enums ============

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SendAction {
    /// Preview the email before sending — ALWAYS do this first
    Preview,
    /// Send the email (requires prior preview)
    Confirm,
    /// Save as draft without sending
    Draft,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SpamAction {
    /// Preview what will happen
    Preview,
    /// Confirm marking as spam
    Confirm,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ContactDeleteAction {
    /// Preview the deletion and receive a confirmation token
    Preview,
    /// Confirm the deletion with a valid confirmation token
    Confirm,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum CalendarDeleteAction {
    Preview,
    Confirm,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum EventDeleteAction {
    Preview,
    Confirm,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GroupDeleteAction {
    /// Preview the deletion and receive a confirmation token
    Preview,
    /// Confirm the deletion with a valid confirmation token
    Confirm,
}

// ============ Result Types ============

#[derive(SimpleObject)]
#[graphql(name = "ComposeResult")]
pub struct GqlComposeResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The email ID (for confirm/draft actions)
    pub email_id: Option<String>,
    /// Preview text (for preview action)
    pub preview: Option<String>,
    /// Confirmation token — returned by PREVIEW, required by CONFIRM/DRAFT
    pub confirmation_token: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "Status")]
pub struct GqlStatus {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "ContactMutationResult")]
pub struct GqlContactMutationResult {
    pub success: bool,
    pub contact: Option<GqlContact>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "ContactDeleteResult")]
pub struct GqlContactDeleteResult {
    pub success: bool,
    pub deleted_id: Option<String>,
    pub preview: Option<String>,
    pub confirmation_token: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "GroupMutationResult")]
pub struct GqlGroupMutationResult {
    pub success: bool,
    pub group: Option<GqlContactGroup>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "GroupDeleteResult")]
pub struct GqlGroupDeleteResult {
    pub success: bool,
    pub deleted_id: Option<String>,
    pub preview: Option<String>,
    pub confirmation_token: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "CalendarMutationResult")]
pub struct GqlCalendarMutationResult {
    pub success: bool,
    pub calendar: Option<GqlCalendar>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "CalendarDeleteResult")]
pub struct GqlCalendarDeleteResult {
    pub success: bool,
    pub deleted_id: Option<String>,
    pub preview: Option<String>,
    pub confirmation_token: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "EventMutationResult")]
pub struct GqlEventMutationResult {
    pub success: bool,
    pub event: Option<GqlCalendarEvent>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "EventDeleteResult")]
pub struct GqlEventDeleteResult {
    pub success: bool,
    pub deleted_id: Option<String>,
    pub preview: Option<String>,
    pub confirmation_token: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Thread result containing all emails in a conversation with full content.
/// Each email has lazy attachment content resolution — only fetched when queried.
pub struct GqlThread {
    pub emails: Vec<Email>,
    pub total: usize,
}

#[Object(name = "Thread")]
impl GqlThread {
    async fn emails(&self) -> Vec<GqlEmail> {
        self.emails.iter().cloned().map(GqlEmail::new).collect()
    }
    async fn total(&self) -> usize {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_email_with_from(count: usize) -> Email {
        let from_addrs: Option<Vec<EmailAddress>> = if count > 0 {
            Some(
                (0..count)
                    .map(|i| EmailAddress {
                        name: Some(format!("Name {i}")),
                        email: format!("addr{i}@example.com"),
                    })
                    .collect(),
            )
        } else {
            None
        };
        Email {
            id: "test-id".to_string(),
            blob_id: None,
            thread_id: None,
            mailbox_ids: HashMap::new(),
            keywords: HashMap::new(),
            size: 0,
            received_at: None,
            message_id: None,
            in_reply_to: None,
            references: None,
            from: from_addrs,
            to: None,
            cc: None,
            bcc: None,
            reply_to: None,
            subject: None,
            sent_at: None,
            preview: None,
            has_attachment: false,
            text_body: None,
            html_body: None,
            attachments: None,
            body_values: None,
        }
    }

    #[test]
    fn gqlemail_addresses_are_arc_shared() {
        let email = make_email_with_from(3);
        let g = GqlEmail::new(email);
        // from has 3 elements precomputed at construction
        assert_eq!(g.from.len(), 3);
        // Cloning the Arc increments the strong count
        let _h1 = Arc::clone(&g.from);
        let _h2 = Arc::clone(&g.from);
        assert!(Arc::strong_count(&g.from) >= 3);
    }

    #[test]
    fn gqlemail_empty_to_is_arc_shared() {
        let email = make_email_with_from(0);
        let g = GqlEmail::new(email);
        assert_eq!(g.to.len(), 0);
        let _h = Arc::clone(&g.to);
        assert!(Arc::strong_count(&g.to) >= 2);
    }

    #[test]
    fn gql_contact_group_from_maps_all_fields() {
        use crate::carddav::ContactGroup;
        let group = ContactGroup {
            id: "uid-1".to_string(),
            name: "Test Group".to_string(),
            member_uids: vec!["uid-a".to_string(), "uid-b".to_string()],
            href: Some("/addressbooks/user/default/group-1.vcf".to_string()),
            etag: Some("\"abc123\"".to_string()),
        };
        let gql: GqlContactGroup = GqlContactGroup::from(group);
        assert_eq!(gql.id, "uid-1");
        assert_eq!(gql.name, "Test Group");
        assert_eq!(gql.member_count, 2);
        assert!(gql.members.is_empty());
        assert_eq!(gql.href.as_deref(), Some("/addressbooks/user/default/group-1.vcf"));
        assert_eq!(gql.etag.as_deref(), Some("\"abc123\""));
    }

    #[test]
    fn gql_contact_group_from_empty_member_uids() {
        use crate::carddav::ContactGroup;
        let group = ContactGroup {
            id: "uid-empty".to_string(),
            name: "Empty Group".to_string(),
            member_uids: vec![],
            href: None,
            etag: None,
        };
        let gql: GqlContactGroup = GqlContactGroup::from(group);
        assert_eq!(gql.member_count, 0);
        assert!(gql.members.is_empty());
        assert!(gql.href.is_none());
        assert!(gql.etag.is_none());
    }
}
