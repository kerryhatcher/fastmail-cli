//! CalDAV client for Fastmail calendars and events.
//!
//! Mirrors the existing CardDAV layering: raw HTTP/WebDAV transport here,
//! thin CLI and MCP surfaces elsewhere.

use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike,
    Utc,
};
use futures::future::join_all;
use reqwest::{
    Client, Method,
    header::{ETAG, HeaderMap, HeaderName, IF_MATCH, IF_NONE_MATCH, LOCATION},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};
pub use uuid::Uuid;

use crate::error::{Error, Result};

const CALDAV_BASE: &str = "https://caldav.fastmail.com";
const DAV_NS: &str = "DAV:";
const CALDAV_NS: &str = "urn:ietf:params:xml:ns:caldav";
const APPLE_ICAL_NS: &str = "http://apple.com/ns/ical/";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub href: String,
    pub etag: Option<String>,
    pub ctag: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EventDateTime {
    pub value: String,
    pub timezone: Option<String>,
    pub all_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub partstat: Option<String>,
    pub rsvp: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EventRecurrence {
    pub frequency: String,
    pub interval: Option<u32>,
    pub count: Option<u32>,
    pub until: Option<String>,
    pub by_day: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EventReminder {
    pub minutes_before: i32,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CalendarEvent {
    pub id: String,
    pub calendar_id: String,
    pub calendar_name: Option<String>,
    pub href: Option<String>,
    pub etag: Option<String>,
    pub title: String,
    pub start: EventDateTime,
    pub end: EventDateTime,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Vec<EventAttendee>,
    pub recurrence: Option<EventRecurrence>,
    pub reminders: Vec<EventReminder>,
}

#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub calendar_id: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

pub struct CalDavClient {
    client: Client,
    username: String,
    app_password: String,
    base_url: String,
}

impl CalDavClient {
    /// Create a new CalDAV client pointing at the production Fastmail server.
    pub fn new(username: String, app_password: String) -> Result<Self> {
        Self::new_with_base_url(username, app_password, CALDAV_BASE.to_string())
    }

    /// Create a new CalDAV client with a custom base URL (for testing).
    pub fn new_with_base_url(
        username: String,
        app_password: String,
        base_url: String,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Config(format!("HTTP client builder failed: {e}")))?;
        Ok(Self {
            client,
            username,
            app_password,
            base_url,
        })
    }

    #[instrument(skip(self))]
    pub async fn discover_calendar_home(&self) -> Result<String> {
        if let Ok(principal_home) = self.discover_calendar_home_via_principal().await {
            return Ok(principal_home);
        }

        Ok(format!("/dav/calendars/user/{}/", self.username))
    }

    #[instrument(skip(self))]
    async fn discover_calendar_home_via_principal(&self) -> Result<String> {
        let principal_href = format!("/dav/principals/user/{}/", self.username);
        let url = format!("{}{principal_href}", self.base_url);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <c:calendar-home-set/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "0")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CalDAV calendar-home discovery failed: {status} - {text}"
            )));
        }

        parse_calendar_home_response(&text)?
            .ok_or_else(|| Error::Server("CalDAV calendar-home-set missing in discovery".into()))
    }

    #[instrument(skip(self))]
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let home = self.discover_calendar_home().await?;
        let url = format!("{}{home}", self.base_url);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:apple="http://apple.com/ns/ical/">
  <d:prop>
    <d:displayname/>
    <d:resourcetype/>
    <d:getetag/>
    <c:supported-calendar-component-set/>
    <apple:calendar-color/>
    <apple:calendar-order/>
    <apple:calendar-description/>
    <cs:getctag xmlns:cs="http://calendarserver.org/ns/"/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, "PROPFIND list_calendars response");

        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CalDAV calendar PROPFIND failed: {status} - {text}"
            )));
        }

        parse_calendars_response(&text)
    }

    #[instrument(skip(self))]
    pub async fn default_calendar(&self) -> Result<Calendar> {
        let calendars = self.list_calendars().await?;
        calendars
            .iter()
            .find(|calendar| calendar.is_default)
            .cloned()
            .or_else(|| calendars.into_iter().next())
            .ok_or_else(|| Error::Server("No calendars found".into()))
    }

    #[instrument(skip(self))]
    pub async fn get_calendar_by_id(&self, calendar_id: &str) -> Result<Calendar> {
        self.list_calendars()
            .await?
            .into_iter()
            .find(|calendar| calendar.id == calendar_id)
            .ok_or_else(|| Error::CalendarNotFound(calendar_id.to_string()))
    }

    #[instrument(skip(self))]
    pub async fn create_calendar(&self, name: &str, color: Option<&str>) -> Result<Calendar> {
        let home = self.discover_calendar_home().await?;
        let slug = slugify(name);
        let href = format!(
            "{}/{slug}-{}",
            home.trim_end_matches('/'),
            Uuid::new_v4().simple()
        ) + "/";
        let url = format!("{}{href}", self.base_url);
        let body = mkcalendar_body(name, color);

        let response = self
            .client
            .request(Method::from_bytes(b"MKCALENDAR").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        if !matches!(
            status,
            reqwest::StatusCode::CREATED
                | reqwest::StatusCode::OK
                | reqwest::StatusCode::NO_CONTENT
        ) {
            return Err(Error::Server(format!(
                "CalDAV MKCALENDAR failed: {status} - {text}"
            )));
        }

        let created_href = extract_location_path(&headers).unwrap_or(href.clone());
        Ok(Calendar {
            id: resource_id_from_href(&created_href),
            name: name.to_string(),
            color: color.map(ToString::to_string),
            description: None,
            href: created_href,
            etag: header_value(&headers, ETAG),
            ctag: None,
            is_default: false,
        })
    }

    #[instrument(skip(self))]
    pub async fn update_calendar(
        &self,
        calendar: &Calendar,
        name: Option<&str>,
        color: Option<&str>,
    ) -> Result<Calendar> {
        let url = format!("{}{}", self.base_url, calendar.href);
        let body = proppatch_calendar_body(
            name.unwrap_or(&calendar.name),
            color.or(calendar.color.as_deref()),
        );
        let mut request = self
            .client
            .request(Method::from_bytes(b"PROPPATCH").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml");

        if let Some(etag) = &calendar.etag {
            request = request.header(IF_MATCH, etag);
        }

        let response = request.body(body).send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;

        if !matches!(
            status,
            reqwest::StatusCode::OK
                | reqwest::StatusCode::NO_CONTENT
                | reqwest::StatusCode::MULTI_STATUS
        ) {
            return Err(map_calendar_write_error(calendar, status, &headers, &text));
        }

        let mut updated = calendar.clone();
        updated.name = name.unwrap_or(&calendar.name).to_string();
        updated.color = color
            .map(ToString::to_string)
            .or_else(|| calendar.color.clone());
        updated.etag = header_value(&headers, ETAG).or_else(|| calendar.etag.clone());
        Ok(updated)
    }

    #[instrument(skip(self))]
    pub async fn delete_calendar(&self, calendar: &Calendar) -> Result<()> {
        let url = format!("{}{}", self.base_url, calendar.href);
        let mut request = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.app_password));
        if let Some(etag) = &calendar.etag {
            request = request.header(IF_MATCH, etag);
        }
        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        if matches!(
            status,
            reqwest::StatusCode::NO_CONTENT
                | reqwest::StatusCode::OK
                | reqwest::StatusCode::ACCEPTED
        ) {
            return Ok(());
        }
        if should_retry_calendar_delete_without_etag(status, calendar.etag.as_deref(), &headers) {
            let response = self
                .client
                .delete(&url)
                .basic_auth(&self.username, Some(&self.app_password))
                .send()
                .await?;
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await?;
            if matches!(
                status,
                reqwest::StatusCode::NO_CONTENT
                    | reqwest::StatusCode::OK
                    | reqwest::StatusCode::ACCEPTED
            ) {
                return Ok(());
            }
            return Err(map_calendar_write_error(calendar, status, &headers, &text));
        }
        Err(map_calendar_write_error(calendar, status, &headers, &text))
    }

    #[instrument(skip(self))]
    pub async fn list_events(&self, query: EventQuery) -> Result<Vec<CalendarEvent>> {
        let calendars = if let Some(calendar_id) = &query.calendar_id {
            vec![self.get_calendar_by_id(calendar_id).await?]
        } else {
            self.list_calendars().await?
        };

        // Build one future per calendar — failures are tolerated (D-01, D-02, D-03)
        let futures: Vec<_> = calendars
            .iter()
            .map(|calendar| {
                let cal = calendar.clone();
                async move {
                    let href = cal.href.clone();
                    self.list_events_in_calendar(&cal, query.start, query.end)
                        .await
                        .map(|events| (href, events))
                }
            })
            .collect();

        let results = join_all(futures).await;
        let mut all_events = collect_partial_events(results);

        all_events.sort_by(|a, b| a.start.value.cmp(&b.start.value));
        Ok(all_events)
    }

    #[instrument(skip(self))]
    async fn list_events_in_calendar(
        &self,
        calendar: &Calendar,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<CalendarEvent>> {
        let url = format!("{}{}", self.base_url, calendar.href);
        let body = calendar_query_body(start, end);
        let response = self
            .client
            .request(Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!(status = %status, calendar = %calendar.id, "REPORT list_events response");
        if !status.is_success() && status.as_u16() != 207 {
            return Err(Error::Server(format!(
                "CalDAV event REPORT failed for calendar {}: {} - {}",
                calendar.id, status, text
            )));
        }

        parse_events_response_for_range(&text, calendar, start, end)
    }

    #[instrument(skip(self))]
    pub async fn get_event_by_id(
        &self,
        event_id: &str,
        calendar_id: Option<&str>,
    ) -> Result<CalendarEvent> {
        let calendars = if let Some(calendar_id) = calendar_id {
            vec![self.get_calendar_by_id(calendar_id).await?]
        } else {
            self.list_calendars().await?
        };

        let report_body = build_uid_report_body(event_id);

        // Issue one REPORT per calendar concurrently (D-04)
        let futures: Vec<_> = calendars
            .iter()
            .map(|calendar| {
                let cal = calendar.clone();
                let body = report_body.clone();
                async move {
                    let url = format!("{}{}", self.base_url, cal.href);
                    let response = self
                        .client
                        .request(Method::from_bytes(b"REPORT").unwrap(), &url)
                        .basic_auth(&self.username, Some(&self.app_password))
                        .header("Content-Type", "application/xml")
                        .header("Depth", "1")
                        .body(body)
                        .send()
                        .await?;
                    let status = response.status();
                    let text = response.text().await?;
                    Ok::<(reqwest::StatusCode, String, Calendar), crate::error::Error>((
                        status, text, cal,
                    ))
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut needs_fallback = false;
        for result in results {
            match result {
                Ok((status, text, cal)) => {
                    if status == reqwest::StatusCode::BAD_REQUEST
                        || status.as_u16() == 501
                    {
                        // Server doesn't support UID REPORT — mark for fallback (D-05)
                        needs_fallback = true;
                    } else if status.is_success() || status.as_u16() == 207 {
                        // Parse the multistatus response for matching VEVENTs
                        if let Ok(events) = parse_events_response_for_range(
                            &text,
                            &cal,
                            None,
                            None,
                        )
                            && let Some(event) =
                                events.into_iter().find(|e| e.id == event_id)
                        {
                            return Ok(event);
                        }
                    } else {
                        warn!(
                            calendar = %cal.id,
                            status = %status,
                            "CalDAV UID REPORT returned unexpected status"
                        );
                    }
                }
                Err(e) => {
                    warn!(error = %e, "CalDAV UID REPORT request failed");
                }
            }
        }

        // Fall back to full-fetch if REPORT unsupported (D-05)
        if needs_fallback {
            warn!(uid = %event_id, "UID REPORT unsupported, falling back to full fetch");
            return self.get_event_by_id_full_fetch(event_id, calendar_id).await;
        }

        Err(Error::EventNotFound(event_id.to_string()))
    }

    /// Full-fetch fallback for get_event_by_id when UID REPORT is unsupported (D-05).
    async fn get_event_by_id_full_fetch(
        &self,
        event_id: &str,
        calendar_id: Option<&str>,
    ) -> Result<CalendarEvent> {
        let calendars = if let Some(calendar_id) = calendar_id {
            vec![self.get_calendar_by_id(calendar_id).await?]
        } else {
            self.list_calendars().await?
        };

        for calendar in calendars {
            let events = self.list_events_in_calendar(&calendar, None, None).await?;
            if let Some(event) = events.into_iter().find(|event| event.id == event_id) {
                return Ok(event);
            }
        }

        Err(Error::EventNotFound(event_id.to_string()))
    }

    #[instrument(skip(self, event))]
    pub async fn create_event(
        &self,
        calendar_id: Option<&str>,
        event: &CalendarEvent,
    ) -> Result<CalendarEvent> {
        let calendar = match calendar_id {
            Some(calendar_id) => self.get_calendar_by_id(calendar_id).await?,
            None => self.default_calendar().await?,
        };
        let href = build_event_href(&calendar.href, &event.id);
        let url = format!("{}{href}", self.base_url);
        let body = serialize_ical_event(event);
        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/calendar; charset=utf-8")
            .header(IF_NONE_MATCH, "*")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        if !matches!(
            status,
            reqwest::StatusCode::CREATED
                | reqwest::StatusCode::NO_CONTENT
                | reqwest::StatusCode::OK
        ) {
            return Err(map_event_write_error(event, None, status, &headers, &text));
        }

        let mut created = event.clone();
        created.calendar_id = calendar.id.clone();
        created.calendar_name = Some(calendar.name.clone());
        created.href = Some(extract_location_path(&headers).unwrap_or(href));
        created.etag = header_value(&headers, ETAG);
        Ok(created)
    }

    #[instrument(skip(self, event))]
    pub async fn update_event(
        &self,
        event: &CalendarEvent,
        previous_etag: &str,
    ) -> Result<CalendarEvent> {
        let href = event
            .href
            .clone()
            .ok_or_else(|| Error::EventNotFound(event.id.clone()))?;
        let url = format!("{}{href}", self.base_url);
        let body = serialize_ical_event(event);
        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/calendar; charset=utf-8")
            .header(IF_MATCH, previous_etag)
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        if !matches!(
            status,
            reqwest::StatusCode::OK
                | reqwest::StatusCode::NO_CONTENT
                | reqwest::StatusCode::CREATED
        ) {
            return Err(map_event_write_error(
                event,
                Some(previous_etag),
                status,
                &headers,
                &text,
            ));
        }

        let mut updated = event.clone();
        updated.etag = header_value(&headers, ETAG).or_else(|| event.etag.clone());
        Ok(updated)
    }

    #[instrument(skip(self))]
    pub async fn delete_event(&self, event: &CalendarEvent) -> Result<()> {
        let href = event
            .href
            .as_deref()
            .ok_or_else(|| Error::EventNotFound(event.id.clone()))?;
        let etag = event.etag.as_deref().ok_or_else(|| Error::EventConflict {
            id: event.id.clone(),
            sent_etag: String::new(),
            server_etag: None,
        })?;

        let url = format!("{}{href}", self.base_url);
        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header(IF_MATCH, etag)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        if matches!(
            status,
            reqwest::StatusCode::NO_CONTENT
                | reqwest::StatusCode::OK
                | reqwest::StatusCode::ACCEPTED
        ) {
            return Ok(());
        }

        Err(map_event_write_error(
            event,
            Some(etag),
            status,
            &headers,
            &text,
        ))
    }
}

/// Flatten results from concurrent per-calendar fetches, logging warnings for failures.
///
/// Returns events from successful calendars only. If all calendars fail, returns
/// an empty `Vec` — never an `Err` — so callers always get a partial/empty result.
fn collect_partial_events(
    results: Vec<Result<(String, Vec<CalendarEvent>)>>,
) -> Vec<CalendarEvent> {
    let mut all = Vec::new();
    for result in results {
        match result {
            Ok((_, events)) => all.extend(events),
            Err(e) => {
                warn!(error = %e, "CalDAV list_events: calendar fetch failed");
            }
        }
    }
    all
}

fn parse_calendar_home_response(xml: &str) -> Result<Option<String>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

    Ok(doc
        .descendants()
        .find(|node| node.has_tag_name((CALDAV_NS, "calendar-home-set")))
        .and_then(|node| {
            node.descendants()
                .find(|child| child.has_tag_name((DAV_NS, "href")))
                .and_then(|child| child.text())
                .map(ToString::to_string)
        }))
}

fn parse_calendars_response(xml: &str) -> Result<Vec<Calendar>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

    let mut calendars = Vec::new();
    let mut default_id = None::<String>;
    let mut best_order = None::<u32>;

    for response in doc
        .descendants()
        .filter(|node| node.has_tag_name((DAV_NS, "response")))
    {
        let href = response
            .descendants()
            .find(|node| node.has_tag_name((DAV_NS, "href")))
            .and_then(|node| node.text())
            .unwrap_or_default()
            .to_string();

        let is_calendar = response
            .descendants()
            .any(|node| node.has_tag_name((CALDAV_NS, "calendar")));
        if !is_calendar || href.is_empty() {
            continue;
        }

        let name = response
            .descendants()
            .find(|node| node.has_tag_name((DAV_NS, "displayname")))
            .and_then(|node| node.text())
            .map(ToString::to_string)
            .unwrap_or_else(|| resource_id_from_href(&href));
        let color = response
            .descendants()
            .find(|node| node.has_tag_name((APPLE_ICAL_NS, "calendar-color")))
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let description = response
            .descendants()
            .find(|node| node.has_tag_name((APPLE_ICAL_NS, "calendar-description")))
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let etag = response
            .descendants()
            .find(|node| node.has_tag_name((DAV_NS, "getetag")))
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let ctag = response
            .descendants()
            .find(|node| node.tag_name().name() == "getctag")
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let order = response
            .descendants()
            .find(|node| node.has_tag_name((APPLE_ICAL_NS, "calendar-order")))
            .and_then(|node| node.text())
            .and_then(|text| text.parse::<u32>().ok());

        let is_likely_default = href.contains("/Default/")
            || name.eq_ignore_ascii_case("default")
            || name.eq_ignore_ascii_case("calendar");

        let calendar_id = resource_id_from_href(&href);
        calendars.push(Calendar {
            id: calendar_id.clone(),
            name,
            color,
            description,
            href,
            etag,
            ctag,
            is_default: false,
        });

        if is_likely_default {
            default_id = Some(calendar_id);
        } else if let Some(order) = order
            && best_order.is_none_or(|best| order < best)
        {
            best_order = Some(order);
            default_id = Some(calendar_id);
        }
    }

    calendars.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    if let Some(default_id) = default_id
        && let Some(calendar) = calendars
            .iter_mut()
            .find(|calendar| calendar.id == default_id)
    {
        calendar.is_default = true;
    } else if let Some(first) = calendars.first_mut() {
        first.is_default = true;
    }

    Ok(calendars)
}

fn parse_events_response_for_range(
    xml: &str,
    calendar: &Calendar,
    range_start: Option<DateTime<Utc>>,
    range_end: Option<DateTime<Utc>>,
) -> Result<Vec<CalendarEvent>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| Error::Server(format!("Failed to parse XML: {e}")))?;

    let mut events = Vec::new();
    for response in doc
        .descendants()
        .filter(|node| node.has_tag_name((DAV_NS, "response")))
    {
        let href = response
            .descendants()
            .find(|node| node.has_tag_name((DAV_NS, "href")))
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let etag = response
            .descendants()
            .find(|node| node.has_tag_name((DAV_NS, "getetag")))
            .and_then(|node| node.text())
            .map(ToString::to_string);
        let ical = response
            .descendants()
            .find(|node| node.has_tag_name((CALDAV_NS, "calendar-data")))
            .and_then(|node| node.text());
        if let Some(ical) = ical
            && let Some(event) = parse_ical_event(
                ical,
                href,
                etag,
                calendar.id.clone(),
                Some(calendar.name.clone()),
            )
        {
            if let Some((range_start, range_end)) = range_start.zip(range_end) {
                events.extend(expand_event_for_range(&event, range_start, range_end));
            } else {
                events.push(event);
            }
        }
    }
    Ok(events)
}

fn parse_ical_event(
    ical: &str,
    href: Option<String>,
    etag: Option<String>,
    calendar_id: String,
    calendar_name: Option<String>,
) -> Option<CalendarEvent> {
    let unfolded = unfold_ical(ical);
    let mut in_event = false;
    let mut in_alarm = false;

    let mut id = String::new();
    let mut title = String::new();
    let mut start = EventDateTime::default();
    let mut end = EventDateTime::default();
    let mut location = None;
    let mut description = None;
    let mut attendees = Vec::new();
    let mut recurrence = None;
    let mut reminders = Vec::new();
    let mut current_alarm = EventReminder::default();

    for raw_line in unfolded.lines() {
        let line = raw_line.trim();
        if line == "BEGIN:VEVENT" {
            in_event = true;
            continue;
        }
        if line == "END:VEVENT" {
            break;
        }
        if !in_event {
            continue;
        }
        if line == "BEGIN:VALARM" {
            in_alarm = true;
            current_alarm = EventReminder::default();
            continue;
        }
        if line == "END:VALARM" {
            in_alarm = false;
            reminders.push(current_alarm.clone());
            continue;
        }

        if in_alarm {
            if line.starts_with("ACTION") {
                current_alarm.action = Some(extract_ical_value(line));
            } else if line.starts_with("TRIGGER") {
                current_alarm.minutes_before = parse_trigger_minutes(&extract_ical_value(line));
            }
            continue;
        }

        if line.starts_with("UID") {
            id = extract_ical_value(line);
        } else if line.starts_with("SUMMARY") {
            title = extract_ical_value(line);
        } else if line.starts_with("DTSTART") {
            start = parse_event_datetime(line);
        } else if line.starts_with("DTEND") {
            end = parse_event_datetime(line);
        } else if line.starts_with("LOCATION") {
            location = Some(extract_ical_value(line));
        } else if line.starts_with("DESCRIPTION") {
            description = Some(extract_ical_value(line));
        } else if line.starts_with("ATTENDEE") {
            if let Some(attendee) = parse_attendee(line) {
                attendees.push(attendee);
            }
        } else if line.starts_with("RRULE") {
            recurrence = parse_rrule(&extract_ical_value(line));
        }
    }

    if id.is_empty() || title.is_empty() || start.value.is_empty() || end.value.is_empty() {
        return None;
    }

    Some(CalendarEvent {
        id,
        calendar_id,
        calendar_name,
        href,
        etag,
        title,
        start,
        end,
        location,
        description,
        attendees,
        recurrence,
        reminders,
    })
}

pub fn serialize_ical_event(event: &CalendarEvent) -> String {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//fastmail-cli//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}", escape_ical_value(&event.id)),
        format!("DTSTAMP:{}", Utc::now().format("%Y%m%dT%H%M%SZ")),
        format!("SUMMARY:{}", escape_ical_value(&event.title)),
        serialize_datetime_property("DTSTART", &event.start),
        serialize_datetime_property("DTEND", &event.end),
    ];

    if let Some(location) = &event.location
        && !location.is_empty()
    {
        lines.push(format!("LOCATION:{}", escape_ical_value(location)));
    }
    if let Some(description) = &event.description
        && !description.is_empty()
    {
        lines.push(format!("DESCRIPTION:{}", escape_ical_value(description)));
    }
    for attendee in &event.attendees {
        lines.push(serialize_attendee(attendee));
    }
    if let Some(recurrence) = &event.recurrence {
        lines.push(format!("RRULE:{}", serialize_rrule(recurrence)));
    }
    for reminder in &event.reminders {
        lines.push("BEGIN:VALARM".to_string());
        lines.push(format!(
            "ACTION:{}",
            reminder.action.clone().unwrap_or_else(|| "DISPLAY".into())
        ));
        lines.push(format!(
            "TRIGGER:{}",
            serialize_trigger_minutes(reminder.minutes_before)
        ));
        lines.push("DESCRIPTION:Reminder".to_string());
        lines.push("END:VALARM".to_string());
    }

    lines.push("END:VEVENT".to_string());
    lines.push("END:VCALENDAR".to_string());

    lines.iter().map(|line| fold_ical_line(line)).collect()
}

pub fn default_today_range() -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Local::now();
    let tomorrow = local_from_naive(
        now.date_naive()
            .succ_opt()
            .unwrap_or_else(|| now.date_naive() + Duration::days(1)),
        0,
        0,
        0,
    );
    (now.with_timezone(&Utc), tomorrow.with_timezone(&Utc))
}

pub fn current_week_range() -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Local::now();
    let start_delta = i64::from(now.weekday().num_days_from_monday());
    let start_date = now.date_naive() - Duration::days(start_delta);
    let end_date = start_date + Duration::days(7);
    (
        local_from_naive(start_date, 0, 0, 0).with_timezone(&Utc),
        local_from_naive(end_date, 0, 0, 0).with_timezone(&Utc),
    )
}

pub fn parse_range_start(value: &str) -> Result<DateTime<Utc>> {
    parse_user_range_datetime(value, false)
}

pub fn parse_range_end(value: &str) -> Result<DateTime<Utc>> {
    parse_user_range_datetime(value, true)
}

fn parse_user_range_datetime(value: &str, end_of_day: bool) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let local = if end_of_day {
            local_from_naive(date.succ_opt().unwrap_or(date + Duration::days(1)), 0, 0, 0)
        } else {
            local_from_naive(date, 0, 0, 0)
        };
        return Ok(local.with_timezone(&Utc));
    }

    for fmt in ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, fmt) {
            return Ok(localize_naive(naive).with_timezone(&Utc));
        }
    }

    Err(Error::Config(format!(
        "Invalid date/time '{value}'. Use YYYY-MM-DD or RFC3339."
    )))
}

fn calendar_query_body(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> String {
    let time_range = match (start, end) {
        (Some(start), Some(end)) => format!(
            r#"<c:time-range start="{}" end="{}"/>"#,
            start.format("%Y%m%dT%H%M%SZ"),
            end.format("%Y%m%dT%H%M%SZ")
        ),
        _ => String::new(),
    };

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data/>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        {time_range}
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#
    )
}

fn mkcalendar_body(name: &str, color: Option<&str>) -> String {
    let color_xml = color
        .map(|color| {
            format!(
                "<apple:calendar-color>{}</apple:calendar-color>",
                xml_escape(color)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:mkcalendar xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:apple="http://apple.com/ns/ical/">
  <d:set>
    <d:prop>
      <d:displayname>{}</d:displayname>
      {}
    </d:prop>
  </d:set>
</c:mkcalendar>"#,
        xml_escape(name),
        color_xml
    )
}

fn proppatch_calendar_body(name: &str, color: Option<&str>) -> String {
    let color_xml = color
        .map(|color| {
            format!(
                "<apple:calendar-color>{}</apple:calendar-color>",
                xml_escape(color)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<d:propertyupdate xmlns:d="DAV:" xmlns:apple="http://apple.com/ns/ical/">
  <d:set>
    <d:prop>
      <d:displayname>{}</d:displayname>
      {}
    </d:prop>
  </d:set>
</d:propertyupdate>"#,
        xml_escape(name),
        color_xml
    )
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            prev_dash = false;
            ch.to_ascii_lowercase()
        } else if !prev_dash {
            prev_dash = true;
            '-'
        } else {
            continue;
        };
        slug.push(mapped);
    }
    slug.trim_matches('-').to_string().if_empty("calendar")
}

fn resource_id_from_href(href: &str) -> String {
    href.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("calendar")
        .trim_end_matches(".ics")
        .to_string()
}

fn build_event_href(calendar_href: &str, event_id: &str) -> String {
    format!("{}/{}.ics", calendar_href.trim_end_matches('/'), event_id)
}

fn header_value(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn extract_location_path(headers: &HeaderMap) -> Option<String> {
    let location = header_value(headers, LOCATION)?;
    if let Some(stripped) = location.strip_prefix(CALDAV_BASE) {
        return Some(stripped.to_string());
    }
    if location.starts_with('/') {
        return Some(location);
    }
    None
}

fn map_calendar_write_error(
    calendar: &Calendar,
    status: reqwest::StatusCode,
    headers: &HeaderMap,
    body: &str,
) -> Error {
    match status {
        reqwest::StatusCode::PRECONDITION_FAILED => Error::CalendarConflict {
            id: calendar.id.clone(),
            sent_etag: calendar.etag.clone().unwrap_or_default(),
            server_etag: header_value(headers, ETAG),
        },
        reqwest::StatusCode::NOT_FOUND => Error::CalendarNotFound(calendar.id.clone()),
        _ => Error::Server(format!(
            "CalDAV calendar write failed for {}: {} - {}",
            calendar.id, status, body
        )),
    }
}

fn should_retry_calendar_delete_without_etag(
    status: reqwest::StatusCode,
    sent_etag: Option<&str>,
    headers: &HeaderMap,
) -> bool {
    status == reqwest::StatusCode::PRECONDITION_FAILED
        && sent_etag.is_some()
        && header_value(headers, ETAG).is_none()
}

fn map_event_write_error(
    event: &CalendarEvent,
    sent_etag: Option<&str>,
    status: reqwest::StatusCode,
    headers: &HeaderMap,
    body: &str,
) -> Error {
    match status {
        reqwest::StatusCode::PRECONDITION_FAILED => Error::EventConflict {
            id: event.id.clone(),
            sent_etag: sent_etag.unwrap_or_default().to_string(),
            server_etag: header_value(headers, ETAG),
        },
        reqwest::StatusCode::NOT_FOUND => Error::EventNotFound(event.id.clone()),
        _ => Error::Server(format!(
            "CalDAV event write failed for {}: {} - {}",
            event.id, status, body
        )),
    }
}

fn unfold_ical(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
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

#[cfg(test)]
mod delete_retry_tests {
    use super::*;

    #[test]
    fn calendar_delete_retries_when_server_rejects_if_match_without_replacement_etag() {
        let headers = HeaderMap::new();
        assert!(should_retry_calendar_delete_without_etag(
            reqwest::StatusCode::PRECONDITION_FAILED,
            Some("\"abc\""),
            &headers,
        ));
    }

    #[test]
    fn calendar_delete_does_not_retry_without_sent_etag() {
        let headers = HeaderMap::new();
        assert!(!should_retry_calendar_delete_without_etag(
            reqwest::StatusCode::PRECONDITION_FAILED,
            None,
            &headers,
        ));
    }

    #[test]
    fn calendar_delete_does_not_retry_when_server_supplies_new_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, "\"next\"".parse().unwrap());
        assert!(!should_retry_calendar_delete_without_etag(
            reqwest::StatusCode::PRECONDITION_FAILED,
            Some("\"abc\""),
            &headers,
        ));
    }
}

fn fold_ical_line(line: &str) -> String {
    const FIRST_MAX: usize = 75;
    const CONT_MAX: usize = 74;

    if line.len() <= FIRST_MAX {
        return format!("{line}\r\n");
    }

    let mut result = String::new();
    let mut end = FIRST_MAX.min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    result.push_str(&line[..end]);
    result.push_str("\r\n ");
    let mut pos = end;
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

fn extract_ical_value(line: &str) -> String {
    line.split_once(':')
        .map(|(_, value)| unescape_ical_value(value))
        .unwrap_or_default()
}

fn escape_ical_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn unescape_ical_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
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
            Some('n') | Some('N') => {
                result.push('\n');
                chars.next();
            }
            _ => result.push(ch),
        }
    }
    result
}

fn parse_event_datetime(line: &str) -> EventDateTime {
    let mut timezone = None;
    let mut all_day = false;
    let prop = line.split(':').next().unwrap_or_default();
    for part in prop.split(';').skip(1) {
        if let Some(tz) = part.strip_prefix("TZID=") {
            timezone = Some(tz.to_string());
        }
        if part == "VALUE=DATE" {
            all_day = true;
        }
    }
    let raw = line.split(':').next_back().unwrap_or_default();
    let value = if all_day {
        NaiveDate::parse_from_str(raw, "%Y%m%d")
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| raw.to_string())
    } else if let Some(stripped) = raw.strip_suffix('Z') {
        NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S")
            .map(|naive| format!("{}Z", naive.format("%Y-%m-%dT%H:%M:%S")))
            .unwrap_or_else(|_| raw.to_string())
    } else {
        NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S")
            .map(|naive| naive.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_else(|_| raw.to_string())
    };
    EventDateTime {
        value,
        timezone,
        all_day,
    }
}

fn serialize_datetime_property(name: &str, dt: &EventDateTime) -> String {
    if dt.all_day {
        let value = NaiveDate::parse_from_str(&dt.value, "%Y-%m-%d")
            .map(|date| date.format("%Y%m%d").to_string())
            .unwrap_or_else(|_| dt.value.clone());
        return format!("{name};VALUE=DATE:{value}");
    }

    if dt.value.ends_with('Z') {
        let trimmed = dt.value.trim_end_matches('Z');
        if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
            return format!("{name}:{}Z", parsed.format("%Y%m%dT%H%M%S"));
        }
        if let Ok(parsed) = DateTime::parse_from_rfc3339(&dt.value) {
            return format!(
                "{name}:{}",
                parsed.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
            );
        }
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(&dt.value, "%Y-%m-%dT%H:%M:%S") {
        let value = parsed.format("%Y%m%dT%H%M%S").to_string();
        if let Some(timezone) = &dt.timezone {
            return format!("{name};TZID={timezone}:{value}");
        }
        return format!("{name}:{value}");
    }

    format!("{name}:{}", dt.value)
}

fn parse_attendee(line: &str) -> Option<EventAttendee> {
    let email = line
        .split(':')
        .next_back()
        .map(|value| value.trim_start_matches("mailto:").to_string())?;
    if email.is_empty() {
        return None;
    }

    let prop = line.split(':').next().unwrap_or_default();
    let mut attendee = EventAttendee {
        email,
        ..Default::default()
    };
    for part in prop.split(';').skip(1) {
        if let Some(name) = part.strip_prefix("CN=") {
            attendee.name = Some(unescape_ical_value(name));
        } else if let Some(role) = part.strip_prefix("ROLE=") {
            attendee.role = Some(role.to_string());
        } else if let Some(partstat) = part.strip_prefix("PARTSTAT=") {
            attendee.partstat = Some(partstat.to_string());
        } else if let Some(rsvp) = part.strip_prefix("RSVP=") {
            attendee.rsvp = Some(rsvp.eq_ignore_ascii_case("TRUE"));
        }
    }
    Some(attendee)
}

/// RFC 5545 §3.3.10: FREQ must be one of the seven enumerated literal values.
fn is_valid_freq(s: &str) -> bool {
    matches!(
        s.to_ascii_uppercase().as_str(),
        "SECONDLY" | "MINUTELY" | "HOURLY" | "DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY"
    )
}

/// RFC 5545 §3.3.10: UNTIL is DATE (YYYYMMDD, 8 digits) or DATE-TIME (YYYYMMDDTHHMMSSZ, 16 chars ending Z).
fn is_valid_rrule_until(s: &str) -> bool {
    let is_date = s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit());
    let is_datetime_utc = s.len() == 16
        && s.as_bytes().get(8) == Some(&b'T')
        && s.as_bytes().get(15) == Some(&b'Z')
        && s[0..8].bytes().all(|b| b.is_ascii_digit())
        && s[9..15].bytes().all(|b| b.is_ascii_digit());
    is_date || is_datetime_utc
}

/// RFC 5545 §3.3.10 weekdaynum: [[+-]1DIGIT or 2DIGIT] + SU/MO/TU/WE/TH/FR/SA
fn is_valid_byday(s: &str) -> bool {
    let up = s.to_ascii_uppercase();
    let bytes = up.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    let wd_start = bytes.len() - 2;
    let weekday = &up[wd_start..];
    if !matches!(weekday, "SU" | "MO" | "TU" | "WE" | "TH" | "FR" | "SA") {
        return false;
    }
    let prefix = &up[..wd_start];
    if prefix.is_empty() {
        return true;
    }
    let prefix = prefix
        .strip_prefix('+')
        .or_else(|| prefix.strip_prefix('-'))
        .unwrap_or(prefix);
    !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_digit())
}

/// Normalize and validate an ATTENDEE ROLE parameter value against the RFC 5545 enum.
fn sanitize_role(role: &str) -> Option<String> {
    let up = role.to_ascii_uppercase();
    matches!(
        up.as_str(),
        "CHAIR" | "REQ-PARTICIPANT" | "OPT-PARTICIPANT" | "NON-PARTICIPANT"
    )
    .then_some(up)
}

/// Normalize and validate an ATTENDEE PARTSTAT parameter value against the RFC 5545 enum.
fn sanitize_partstat(partstat: &str) -> Option<String> {
    let up = partstat.to_ascii_uppercase();
    matches!(
        up.as_str(),
        "NEEDS-ACTION"
            | "ACCEPTED"
            | "DECLINED"
            | "TENTATIVE"
            | "DELEGATED"
            | "COMPLETED"
            | "IN-PROCESS"
    )
    .then_some(up)
}

fn serialize_attendee(attendee: &EventAttendee) -> String {
    let mut prop = "ATTENDEE".to_string();
    if let Some(name) = &attendee.name {
        prop.push_str(&format!(";CN={}", escape_ical_value(name)));
    }
    if let Some(role) = attendee.role.as_deref().and_then(sanitize_role) {
        prop.push_str(&format!(";ROLE={role}"));
    }
    if let Some(partstat) = attendee.partstat.as_deref().and_then(sanitize_partstat) {
        prop.push_str(&format!(";PARTSTAT={partstat}"));
    }
    if let Some(rsvp) = attendee.rsvp {
        prop.push_str(&format!(";RSVP={}", if rsvp { "TRUE" } else { "FALSE" }));
    }
    format!("{prop}:mailto:{}", escape_ical_value(&attendee.email))
}

fn parse_rrule(value: &str) -> Option<EventRecurrence> {
    let mut recurrence = EventRecurrence::default();
    for part in value.split(';') {
        let (key, val) = part.split_once('=')?;
        match key {
            "FREQ" => recurrence.frequency = val.to_string(),
            "INTERVAL" => recurrence.interval = val.parse().ok(),
            "COUNT" => recurrence.count = val.parse().ok(),
            "UNTIL" => recurrence.until = Some(val.to_string()),
            "BYDAY" => recurrence.by_day = val.split(',').map(ToString::to_string).collect(),
            _ => {}
        }
    }
    if recurrence.frequency.is_empty() {
        None
    } else {
        Some(recurrence)
    }
}

fn serialize_rrule(recurrence: &EventRecurrence) -> String {
    let freq = if is_valid_freq(&recurrence.frequency) {
        recurrence.frequency.to_ascii_uppercase()
    } else {
        String::new()
    };
    let mut parts = vec![format!("FREQ={freq}")];
    if let Some(interval) = recurrence.interval {
        parts.push(format!("INTERVAL={interval}"));
    }
    if let Some(count) = recurrence.count {
        parts.push(format!("COUNT={count}"));
    }
    if let Some(until) = recurrence.until.as_deref()
        && is_valid_rrule_until(until)
    {
        parts.push(format!("UNTIL={until}"));
    }
    let by_day: Vec<String> = recurrence
        .by_day
        .iter()
        .filter(|d| is_valid_byday(d))
        .map(|d| d.to_ascii_uppercase())
        .collect();
    if !by_day.is_empty() {
        parts.push(format!("BYDAY={}", by_day.join(",")));
    }
    parts.join(";")
}

fn parse_trigger_minutes(trigger: &str) -> i32 {
    if !trigger.starts_with("-PT") {
        return 0;
    }
    let mut minutes = 0;
    let rest = &trigger[3..];
    if let Some(hours) = rest.split('H').next()
        && rest.contains('H')
    {
        minutes += hours.parse::<i32>().unwrap_or(0) * 60;
    }
    if let Some(min_part) = rest.split('H').next_back()
        && let Some(mins) = min_part.strip_suffix('M')
    {
        minutes += mins.parse::<i32>().unwrap_or(0);
    }
    minutes
}

fn serialize_trigger_minutes(minutes_before: i32) -> String {
    let total = minutes_before.abs();
    let hours = total / 60;
    let mins = total % 60;
    match (hours, mins) {
        (0, mins) => format!("-PT{mins}M"),
        (hours, 0) => format!("-PT{hours}H"),
        (hours, mins) => format!("-PT{hours}H{mins}M"),
    }
}

fn expand_event_for_range(
    event: &CalendarEvent,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Vec<CalendarEvent> {
    if let Some(recurrence) = &event.recurrence {
        return expand_recurring_event(event, recurrence, range_start, range_end);
    }

    if event_overlaps_range(event, range_start, range_end) {
        vec![event.clone()]
    } else {
        Vec::new()
    }
}

fn expand_recurring_event(
    event: &CalendarEvent,
    recurrence: &EventRecurrence,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Vec<CalendarEvent> {
    let Some(base_start) = event_start_utc(event) else {
        return Vec::new();
    };
    let Some(base_end) = event_end_utc(event) else {
        return Vec::new();
    };
    let duration = base_end - base_start;
    if duration < Duration::zero() {
        return Vec::new();
    }

    let interval = recurrence.interval.unwrap_or(1).max(1);
    let until = recurrence.until.as_deref().and_then(parse_until_utc);

    let mut occurrences = Vec::new();
    match recurrence.frequency.as_str() {
        "DAILY" => {
            let mut cursor = base_start;
            let mut count = 0_u32;
            loop {
                if beyond_limits(cursor, until, recurrence.count, count) || cursor >= range_end {
                    break;
                }
                if overlaps_occurrence(cursor, cursor + duration, range_start, range_end) {
                    occurrences.push(shift_event_instance(event, cursor, duration));
                }
                cursor += Duration::days(i64::from(interval));
                count += 1;
            }
        }
        "WEEKLY" => {
            let weekdays = if recurrence.by_day.is_empty() {
                vec![weekday_code_from_datetime(base_start)]
            } else {
                recurrence.by_day.clone()
            };

            let mut week_cursor = start_of_week(base_start);
            let mut count = 0_u32;
            loop {
                if week_cursor >= range_end {
                    break;
                }
                let week_index =
                    (week_cursor.date_naive() - start_of_week(base_start).date_naive()).num_days()
                        / 7;
                if week_index >= 0 && week_index % i64::from(interval) == 0 {
                    for weekday in &weekdays {
                        let Some(weekday_number) = weekday_code_to_index(weekday) else {
                            continue;
                        };
                        let candidate = week_cursor + Duration::days(i64::from(weekday_number));
                        if candidate < base_start {
                            continue;
                        }
                        if beyond_limits(candidate, until, recurrence.count, count) {
                            return occurrences;
                        }
                        let candidate_end = candidate + duration;
                        if candidate >= range_end {
                            continue;
                        }
                        if overlaps_occurrence(candidate, candidate_end, range_start, range_end) {
                            occurrences.push(shift_event_instance(event, candidate, duration));
                        }
                        count += 1;
                    }
                }
                week_cursor += Duration::weeks(1);
            }
        }
        "MONTHLY" => {
            let mut count = 0_u32;
            let mut month_index = 0_i32;
            loop {
                let Some(candidate) = add_months_preserving_local(base_start, month_index) else {
                    break;
                };
                if month_index > 0 && !(month_index as u32).is_multiple_of(interval) {
                    month_index += 1;
                    continue;
                }
                if beyond_limits(candidate, until, recurrence.count, count)
                    || candidate >= range_end
                {
                    break;
                }
                if overlaps_occurrence(candidate, candidate + duration, range_start, range_end) {
                    occurrences.push(shift_event_instance(event, candidate, duration));
                }
                month_index += 1;
                count += 1;
            }
        }
        _ => {
            if event_overlaps_range(event, range_start, range_end) {
                occurrences.push(event.clone());
            }
        }
    }

    occurrences
}

fn beyond_limits(
    candidate_start: DateTime<Utc>,
    until: Option<DateTime<Utc>>,
    count_limit: Option<u32>,
    current_count: u32,
) -> bool {
    until.is_some_and(|until| candidate_start > until)
        || count_limit.is_some_and(|limit| current_count >= limit)
}

fn overlaps_occurrence(
    occurrence_start: DateTime<Utc>,
    occurrence_end: DateTime<Utc>,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> bool {
    occurrence_start < range_end && occurrence_end > range_start
}

fn event_overlaps_range(
    event: &CalendarEvent,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> bool {
    event_start_utc(event)
        .zip(event_end_utc(event))
        .is_some_and(|(start, end)| overlaps_occurrence(start, end, range_start, range_end))
}

fn shift_event_instance(
    event: &CalendarEvent,
    occurrence_start: DateTime<Utc>,
    duration: Duration,
) -> CalendarEvent {
    let occurrence_end = occurrence_start + duration;
    let mut shifted = event.clone();
    shifted.start = shift_event_datetime(&event.start, occurrence_start);
    shifted.end = shift_event_datetime(&event.end, occurrence_end);
    shifted
}

fn shift_event_datetime(template: &EventDateTime, instant: DateTime<Utc>) -> EventDateTime {
    if template.all_day {
        return EventDateTime {
            value: instant
                .with_timezone(&Local)
                .date_naive()
                .format("%Y-%m-%d")
                .to_string(),
            timezone: None,
            all_day: true,
        };
    }

    if let Some(timezone) = &template.timezone
        && let Some(offset) = parse_olson_timezone_offset(timezone, instant)
    {
        return EventDateTime {
            value: (instant + offset).format("%Y-%m-%dT%H:%M:%S").to_string(),
            timezone: Some(timezone.clone()),
            all_day: false,
        };
    }

    if template.value.ends_with('Z') {
        EventDateTime {
            value: instant.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            timezone: None,
            all_day: false,
        }
    } else {
        EventDateTime {
            value: instant
                .with_timezone(&Local)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            timezone: template.timezone.clone(),
            all_day: false,
        }
    }
}

fn event_start_utc(event: &CalendarEvent) -> Option<DateTime<Utc>> {
    event_datetime_to_utc(&event.start)
}

fn event_end_utc(event: &CalendarEvent) -> Option<DateTime<Utc>> {
    event_datetime_to_utc(&event.end)
}

fn event_datetime_to_utc(datetime: &EventDateTime) -> Option<DateTime<Utc>> {
    if datetime.all_day {
        let date = NaiveDate::parse_from_str(&datetime.value, "%Y-%m-%d").ok()?;
        return Some(local_from_naive(date, 0, 0, 0).with_timezone(&Utc));
    }

    if datetime.value.ends_with('Z') {
        return DateTime::parse_from_rfc3339(&datetime.value)
            .ok()
            .map(|value| value.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(&datetime.value, "%Y-%m-%dT%H:%M:%S").ok()?;
    if let Some(timezone) = &datetime.timezone
        && let Some(offset) = parse_olson_timezone_offset(timezone, Utc::now())
    {
        return Some((localize_naive(naive) - offset).with_timezone(&Utc));
    }

    Some(localize_naive(naive).with_timezone(&Utc))
}

fn parse_until_utc(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Some(stripped) = value.strip_suffix('Z')
        && let Ok(naive) = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S")
    {
        return Some(Utc.from_utc_datetime(&naive));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y%m%d") {
        return Some(local_from_naive(date.succ_opt()?, 0, 0, 0).with_timezone(&Utc));
    }
    None
}

fn start_of_week(value: DateTime<Utc>) -> DateTime<Utc> {
    value - Duration::days(i64::from(value.weekday().num_days_from_monday()))
}

fn weekday_code_from_datetime(value: DateTime<Utc>) -> String {
    match value.weekday() {
        chrono::Weekday::Mon => "MO",
        chrono::Weekday::Tue => "TU",
        chrono::Weekday::Wed => "WE",
        chrono::Weekday::Thu => "TH",
        chrono::Weekday::Fri => "FR",
        chrono::Weekday::Sat => "SA",
        chrono::Weekday::Sun => "SU",
    }
    .to_string()
}

fn weekday_code_to_index(value: &str) -> Option<u32> {
    match value {
        "MO" => Some(0),
        "TU" => Some(1),
        "WE" => Some(2),
        "TH" => Some(3),
        "FR" => Some(4),
        "SA" => Some(5),
        "SU" => Some(6),
        _ => None,
    }
}

fn add_months_preserving_local(base: DateTime<Utc>, month_offset: i32) -> Option<DateTime<Utc>> {
    let naive = base.naive_utc();
    let year = naive.year();
    let month0 = naive.month0() as i32 + month_offset;
    let new_year = year + month0.div_euclid(12);
    let new_month0 = month0.rem_euclid(12) as u32;
    let new_month = new_month0 + 1;
    let last_day = last_day_of_month(new_year, new_month)?;
    let day = naive.day().min(last_day);
    let new_date = NaiveDate::from_ymd_opt(new_year, new_month, day)?;
    let new_naive = new_date.and_hms_opt(naive.hour(), naive.minute(), naive.second())?;
    Some(Utc.from_utc_datetime(&new_naive))
}

fn last_day_of_month(year: i32, month: u32) -> Option<u32> {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    Some((next_month - Duration::days(1)).day())
}

fn parse_olson_timezone_offset(_timezone: &str, _instant: DateTime<Utc>) -> Option<Duration> {
    None
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escape XML special characters in a UID for embedding in XML text content.
///
/// Handles `&`, `<`, `>`, `"`, and `'` to prevent injection.
fn xml_escape_uid(uid: &str) -> String {
    uid.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Build an RFC 4791 calendar-query REPORT body targeting a specific event UID.
///
/// Uses `collation="i;unicode-casemap"` and `match-type="equals"` for exact UID
/// match as required by D-06. XML special characters in the UID are escaped.
pub(crate) fn build_uid_report_body(uid: &str) -> String {
    let escaped_uid = xml_escape_uid(uid);
    format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:prop-filter name="UID">
          <C:text-match collation="i;unicode-casemap" match-type="equals">{escaped_uid}</C:text-match>
        </C:prop-filter>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#
    )
}

fn local_from_naive(date: NaiveDate, hour: u32, minute: u32, second: u32) -> DateTime<Local> {
    let naive = date
        .and_hms_opt(hour, minute, second)
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).expect("valid fallback time"));
    localize_naive(naive)
}

fn localize_naive(naive: NaiveDateTime) -> DateTime<Local> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Local.from_utc_datetime(&naive),
    }
}

trait EmptyFallback {
    fn if_empty(self, fallback: &str) -> String;
}

impl EmptyFallback for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn test_parse_calendar_home_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:propstat>
      <d:prop>
        <c:calendar-home-set>
          <d:href>/dav/calendars/user/test@example.com/</d:href>
        </c:calendar-home-set>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
        assert_eq!(
            parse_calendar_home_response(xml).unwrap().as_deref(),
            Some("/dav/calendars/user/test@example.com/")
        );
    }

    #[test]
    fn test_parse_calendars_response_marks_default_and_color() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:apple="http://apple.com/ns/ical/">
  <d:response>
    <d:href>/dav/calendars/user/test/Default/</d:href>
    <d:propstat>
      <d:prop>
        <d:displayname>Default</d:displayname>
        <d:getetag>"etag-1"</d:getetag>
        <d:resourcetype><d:collection/><c:calendar/></d:resourcetype>
        <apple:calendar-color>#00ff00</apple:calendar-color>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
        let calendars = parse_calendars_response(xml).unwrap();
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].id, "Default");
        assert_eq!(calendars[0].color.as_deref(), Some("#00ff00"));
        assert!(calendars[0].is_default);
    }

    #[test]
    fn test_parse_and_serialize_event_round_trip() {
        let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:event-1\r\nSUMMARY:Team Sync\r\nDTSTART;TZID=America/New_York:20260403T090000\r\nDTEND;TZID=America/New_York:20260403T093000\r\nLOCATION:Room 1\r\nDESCRIPTION:Agenda\\nLine 2\r\nATTENDEE;CN=Alice;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED:mailto:alice@example.com\r\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT15M\r\nDESCRIPTION:Reminder\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = parse_ical_event(
            ics,
            Some("/dav/calendars/user/test/default/event-1.ics".into()),
            Some("\"etag\"".into()),
            "default".into(),
            Some("Default".into()),
        )
        .unwrap();
        assert_eq!(event.id, "event-1");
        assert_eq!(event.title, "Team Sync");
        assert_eq!(event.start.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(event.attendees.len(), 1);
        assert_eq!(event.reminders[0].minutes_before, 15);

        let serialized = serialize_ical_event(&event);
        assert!(serialized.contains("SUMMARY:Team Sync"));
        assert!(serialized.contains("RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE"));
        assert!(serialized.contains("TRIGGER:-PT15M"));
    }

    #[test]
    fn test_parse_event_datetime_all_day() {
        let parsed = parse_event_datetime("DTSTART;VALUE=DATE:20260403");
        assert!(parsed.all_day);
        assert_eq!(parsed.value, "2026-04-03");
    }

    #[test]
    fn test_serialize_datetime_property_with_timezone() {
        let dt = EventDateTime {
            value: "2026-04-03T09:00:00".into(),
            timezone: Some("America/New_York".into()),
            all_day: false,
        };
        assert_eq!(
            serialize_datetime_property("DTSTART", &dt),
            "DTSTART;TZID=America/New_York:20260403T090000"
        );
    }

    #[test]
    fn test_parse_range_helpers_accept_date_and_rfc3339() {
        assert!(parse_range_start("2026-04-03").is_ok());
        assert!(parse_range_end("2026-04-03T12:00:00Z").is_ok());
    }

    #[test]
    fn test_extract_location_path_handles_absolute_and_relative_urls() {
        let mut headers = HeaderMap::new();
        headers.insert(
            LOCATION,
            HeaderValue::from_static("https://caldav.fastmail.com/dav/calendars/user/test/abc/"),
        );
        assert_eq!(
            extract_location_path(&headers).as_deref(),
            Some("/dav/calendars/user/test/abc/")
        );

        headers.insert(
            LOCATION,
            HeaderValue::from_static("/dav/calendars/user/test/xyz/"),
        );
        assert_eq!(
            extract_location_path(&headers).as_deref(),
            Some("/dav/calendars/user/test/xyz/")
        );
    }

    #[test]
    fn test_recurring_all_day_event_expands_to_matching_occurrence() {
        let event = parse_ical_event(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:payday\r\nSUMMARY:Payday\r\nDTSTART;VALUE=DATE:20260123\r\nDTEND;VALUE=DATE:20260124\r\nRRULE:FREQ=WEEKLY;INTERVAL=2\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            Some("/dav/calendars/user/test/default/payday.ics".into()),
            Some("\"etag\"".into()),
            "default".into(),
            Some("Default".into()),
        )
        .unwrap();

        let range_start = parse_range_start("2026-04-03").unwrap();
        let range_end = parse_range_end("2026-04-03").unwrap();
        let expanded = expand_event_for_range(&event, range_start, range_end);

        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].start.value, "2026-04-03");
        assert_eq!(expanded[0].end.value, "2026-04-04");
    }

    #[test]
    fn test_non_matching_recurring_event_is_filtered_out() {
        let event = parse_ical_event(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:weekly-sync\r\nSUMMARY:Weekly Sync\r\nDTSTART:20260401T130000Z\r\nDTEND:20260401T133000Z\r\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=WE\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            Some("/dav/calendars/user/test/default/weekly-sync.ics".into()),
            Some("\"etag\"".into()),
            "default".into(),
            Some("Default".into()),
        )
        .unwrap();

        let range_start = parse_range_start("2026-04-03").unwrap();
        let range_end = parse_range_end("2026-04-03").unwrap();
        let expanded = expand_event_for_range(&event, range_start, range_end);

        assert!(expanded.is_empty());
    }

    #[test]
    fn test_caldav_client_new_returns_ok() {
        let result = CalDavClient::new("user@example.com".to_string(), "pass".to_string());
        assert!(result.is_ok());
    }

    // ===== build_uid_report_body tests =====

    #[test]
    fn build_uid_report_body_contains_prop_filter_uid() {
        let body = build_uid_report_body("abc-123");
        assert!(
            body.contains(r#"<C:prop-filter name="UID">"#),
            "missing prop-filter UID: {body}"
        );
    }

    #[test]
    fn build_uid_report_body_contains_correct_collation_and_match_type() {
        let body = build_uid_report_body("abc-123");
        assert!(
            body.contains(r#"collation="i;unicode-casemap""#),
            "missing collation: {body}"
        );
        assert!(
            body.contains(r#"match-type="equals""#),
            "missing match-type=equals: {body}"
        );
    }

    #[test]
    fn build_uid_report_body_contains_uid_text() {
        let body = build_uid_report_body("my-uid-value");
        assert!(
            body.contains("my-uid-value"),
            "missing UID text: {body}"
        );
    }

    #[test]
    fn build_uid_report_body_escapes_ampersand_in_uid() {
        let body = build_uid_report_body("a&b");
        assert!(body.contains("a&amp;b"), "missing &amp; escape: {body}");
        assert!(!body.contains("a&b\n"), "unescaped & present: {body}");
    }

    #[test]
    fn build_uid_report_body_escapes_xml_special_chars_in_uid() {
        let body = build_uid_report_body("a&b<c>d\"e'f");
        assert!(body.contains("a&amp;b"), "missing &amp;: {body}");
        assert!(body.contains("&lt;c&gt;"), "missing &lt;&gt;: {body}");
        assert!(body.contains("&quot;e&apos;f"), "missing &quot;&apos;: {body}");
    }

    // ===== collect_partial_events tests =====

    fn make_event(id: &str) -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            calendar_id: "default".to_string(),
            title: id.to_string(),
            start: EventDateTime {
                value: "2026-04-03T09:00:00Z".to_string(),
                timezone: None,
                all_day: false,
            },
            end: EventDateTime {
                value: "2026-04-03T10:00:00Z".to_string(),
                timezone: None,
                all_day: false,
            },
            ..Default::default()
        }
    }

    #[test]
    fn list_events_partial_failure_returns_successes() {
        // 3 calendars: first and third succeed, middle fails
        let results: Vec<Result<(String, Vec<CalendarEvent>)>> = vec![
            Ok(("/cal1".to_string(), vec![make_event("event-a")])),
            Err(Error::Server("connection refused".to_string())),
            Ok(("/cal3".to_string(), vec![make_event("event-c")])),
        ];
        let events = collect_partial_events(results);
        assert_eq!(events.len(), 2);
        let ids: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"event-a"));
        assert!(ids.contains(&"event-c"));
    }

    #[test]
    fn list_events_all_success_flattened_in_order() {
        let results: Vec<Result<(String, Vec<CalendarEvent>)>> = vec![
            Ok(("/cal1".to_string(), vec![make_event("event-a"), make_event("event-b")])),
            Ok(("/cal2".to_string(), vec![make_event("event-c")])),
        ];
        let events = collect_partial_events(results);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].id, "event-a");
        assert_eq!(events[1].id, "event-b");
        assert_eq!(events[2].id, "event-c");
    }

    #[test]
    fn list_events_all_failure_returns_empty() {
        let results: Vec<Result<(String, Vec<CalendarEvent>)>> = vec![
            Err(Error::Server("timeout".to_string())),
            Err(Error::Server("timeout".to_string())),
        ];
        let events = collect_partial_events(results);
        assert!(events.is_empty());
    }
}

#[cfg(test)]
mod sec03_tests {
    use super::*;

    #[test]
    fn test_attendee_email_newline_escaped() {
        let a = EventAttendee {
            email: "a@b.com\nSUMMARY:Injected".into(),
            ..Default::default()
        };
        let s = serialize_attendee(&a);
        assert!(
            s.contains(r"mailto:a@b.com\nSUMMARY:Injected"),
            "got: {s}"
        );
        assert_eq!(s.lines().count(), 1, "must not produce multiple lines: {s}");
    }

    #[test]
    fn test_attendee_role_invalid_dropped() {
        let a = EventAttendee {
            email: "a@b.com".into(),
            role: Some("INJECT;X".into()),
            ..Default::default()
        };
        let s = serialize_attendee(&a);
        assert!(!s.contains("ROLE="), "bad role should be dropped: {s}");
    }

    #[test]
    fn test_attendee_partstat_case_normalized() {
        let a = EventAttendee {
            email: "a@b.com".into(),
            partstat: Some("accepted".into()),
            ..Default::default()
        };
        let s = serialize_attendee(&a);
        assert!(s.contains("PARTSTAT=ACCEPTED"), "got: {s}");
    }

    #[test]
    fn test_rrule_until_invalid_dropped() {
        let r = EventRecurrence {
            frequency: "DAILY".into(),
            until: Some("2026-04-30;INJECT".into()),
            ..Default::default()
        };
        let s = serialize_rrule(&r);
        assert!(!s.contains("UNTIL="), "bad until should be dropped: {s}");
    }

    #[test]
    fn test_rrule_until_valid_kept() {
        let r1 = EventRecurrence {
            frequency: "DAILY".into(),
            until: Some("20260430".into()),
            ..Default::default()
        };
        assert!(serialize_rrule(&r1).contains("UNTIL=20260430"));
        let r2 = EventRecurrence {
            frequency: "DAILY".into(),
            until: Some("20260430T120000Z".into()),
            ..Default::default()
        };
        assert!(serialize_rrule(&r2).contains("UNTIL=20260430T120000Z"));
    }

    #[test]
    fn test_rrule_byday_invalid_dropped() {
        let r = EventRecurrence {
            frequency: "WEEKLY".into(),
            by_day: vec!["MO".into(), "BAD\n".into(), "-1SU".into()],
            ..Default::default()
        };
        let s = serialize_rrule(&r);
        assert!(s.contains("BYDAY=MO,-1SU"), "got: {s}");
    }

    #[test]
    fn test_rrule_freq_invalid_empty() {
        let r = EventRecurrence {
            frequency: "EVIL;X".into(),
            ..Default::default()
        };
        let s = serialize_rrule(&r);
        assert!(
            s.starts_with("FREQ=;") || s == "FREQ=",
            "bad freq should be empty: {s}"
        );
    }
}
