use anyhow::Result as AnyResult;

use crate::caldav::{
    CalDavClient, CalendarEvent, EventAttendee, EventDateTime, EventQuery, EventRecurrence,
    EventReminder, Uuid, current_week_range, default_today_range, parse_range_end,
    parse_range_start,
};
use crate::config::Config;
use crate::models::Output;

#[derive(Debug, Clone, Default)]
pub struct EventInput {
    pub calendar_id: Option<String>,
    pub title: String,
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Vec<EventAttendee>,
    pub recurrence: Option<EventRecurrence>,
    pub reminders: Vec<EventReminder>,
}

#[derive(Debug, Clone, Default)]
pub struct EventPatch {
    pub title: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Option<Vec<EventAttendee>>,
    pub recurrence: Option<EventRecurrence>,
    pub clear_recurrence: bool,
    pub reminders: Option<Vec<EventReminder>>,
    pub clear_reminders: bool,
}

type EventRange = (
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

fn calendar_client() -> crate::error::Result<CalDavClient> {
    let config = Config::load()?;
    let username = config.get_username()?;
    let app_password = config.get_app_password()?;
    Ok(CalDavClient::new(username, app_password))
}

fn normalize_event_datetime(
    value: &str,
    timezone: Option<String>,
) -> crate::error::Result<EventDateTime> {
    if chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        return Ok(EventDateTime {
            value: value.to_string(),
            timezone: None,
            all_day: true,
        });
    }

    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(EventDateTime {
            value: parsed
                .with_timezone(&chrono::Utc)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            timezone: None,
            all_day: false,
        });
    }

    for fmt in ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(value, fmt) {
            return Ok(EventDateTime {
                value: parsed.format("%Y-%m-%dT%H:%M:%S").to_string(),
                timezone,
                all_day: false,
            });
        }
    }

    Err(crate::error::Error::Config(format!(
        "Invalid event date/time '{value}'. Use YYYY-MM-DD, YYYY-MM-DDTHH:MM[:SS], or RFC3339."
    )))
}

pub fn build_event(input: EventInput) -> crate::error::Result<CalendarEvent> {
    let start = normalize_event_datetime(&input.start, input.timezone.clone())?;
    let end = normalize_event_datetime(&input.end, input.timezone.clone())?;
    Ok(CalendarEvent {
        id: Uuid::new_v4().to_string(),
        calendar_id: input.calendar_id.unwrap_or_default(),
        calendar_name: None,
        href: None,
        etag: None,
        title: input.title,
        start,
        end,
        location: input.location.filter(|value| !value.is_empty()),
        description: input.description.filter(|value| !value.is_empty()),
        attendees: input.attendees,
        recurrence: input.recurrence,
        reminders: input.reminders,
    })
}

pub fn apply_event_patch(
    existing: &CalendarEvent,
    patch: EventPatch,
) -> crate::error::Result<CalendarEvent> {
    let timezone = if patch.timezone.is_some() {
        patch.timezone.clone()
    } else {
        existing.start.timezone.clone()
    };
    let start = match patch.start {
        Some(start) => normalize_event_datetime(&start, timezone.clone())?,
        None => {
            let mut start = existing.start.clone();
            if patch.timezone.is_some() && !start.all_day {
                start.timezone = timezone.clone();
            }
            start
        }
    };
    let end = match patch.end {
        Some(end) => normalize_event_datetime(&end, timezone)?,
        None => {
            let mut end = existing.end.clone();
            if patch.timezone.is_some() && !end.all_day {
                end.timezone = patch.timezone.clone();
            }
            end
        }
    };

    Ok(CalendarEvent {
        id: existing.id.clone(),
        calendar_id: existing.calendar_id.clone(),
        calendar_name: existing.calendar_name.clone(),
        href: existing.href.clone(),
        etag: existing.etag.clone(),
        title: patch.title.unwrap_or_else(|| existing.title.clone()),
        start,
        end,
        location: patch
            .location
            .map(|value| (!value.is_empty()).then_some(value))
            .unwrap_or_else(|| existing.location.clone()),
        description: patch
            .description
            .map(|value| (!value.is_empty()).then_some(value))
            .unwrap_or_else(|| existing.description.clone()),
        attendees: patch
            .attendees
            .unwrap_or_else(|| existing.attendees.clone()),
        recurrence: if patch.clear_recurrence {
            None
        } else {
            patch.recurrence.or_else(|| existing.recurrence.clone())
        },
        reminders: if patch.clear_reminders {
            Vec::new()
        } else {
            patch
                .reminders
                .unwrap_or_else(|| existing.reminders.clone())
        },
    })
}

fn resolve_event_list_range(
    start: Option<&str>,
    end: Option<&str>,
    week: bool,
) -> crate::error::Result<EventRange> {
    if week {
        let (start, end) = current_week_range();
        return Ok((Some(start), Some(end)));
    }

    match (start, end) {
        (Some(start), Some(end)) => {
            Ok((Some(parse_range_start(start)?), Some(parse_range_end(end)?)))
        }
        (None, None) => {
            let (start, end) = default_today_range();
            Ok((Some(start), Some(end)))
        }
        _ => Err(crate::error::Error::Config(
            "Explicit event ranges require both start and end.".to_string(),
        )),
    }
}

pub async fn list_events_record(
    calendar_id: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    week: bool,
) -> crate::error::Result<Vec<CalendarEvent>> {
    let (range_start, range_end) = resolve_event_list_range(start, end, week)?;
    let client = calendar_client()?;

    client
        .list_events(EventQuery {
            calendar_id: calendar_id.map(ToString::to_string),
            start: range_start,
            end: range_end,
        })
        .await
}

pub async fn get_event_record(
    event_id: &str,
    calendar_id: Option<&str>,
) -> crate::error::Result<CalendarEvent> {
    let client = calendar_client()?;
    client.get_event_by_id(event_id, calendar_id).await
}

pub async fn create_event_record(input: EventInput) -> crate::error::Result<CalendarEvent> {
    let client = calendar_client()?;
    let event = build_event(input.clone())?;
    client
        .create_event(input.calendar_id.as_deref(), &event)
        .await
}

pub async fn update_event_record(
    event_id: &str,
    calendar_id: Option<&str>,
    patch: EventPatch,
) -> crate::error::Result<CalendarEvent> {
    let client = calendar_client()?;
    let existing = client.get_event_by_id(event_id, calendar_id).await?;
    let previous_etag =
        existing
            .etag
            .clone()
            .ok_or_else(|| crate::error::Error::EventConflict {
                id: event_id.to_string(),
                sent_etag: String::new(),
                server_etag: None,
            })?;
    let updated = apply_event_patch(&existing, patch)?;
    client.update_event(&updated, &previous_etag).await
}

pub async fn delete_event_record(
    event_id: &str,
    calendar_id: Option<&str>,
) -> crate::error::Result<()> {
    let client = calendar_client()?;
    let existing = client.get_event_by_id(event_id, calendar_id).await?;
    client.delete_event(&existing).await
}

pub async fn list_events(
    calendar_id: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    week: bool,
) -> AnyResult<()> {
    let events = list_events_record(calendar_id, start, end, week).await?;
    Output::success(events).print();
    Ok(())
}

pub async fn get_event(event_id: &str, calendar_id: Option<&str>) -> AnyResult<()> {
    let event = get_event_record(event_id, calendar_id).await?;
    Output::success(event).print();
    Ok(())
}

pub async fn create_event(input: EventInput) -> AnyResult<()> {
    let event = create_event_record(input).await?;
    Output {
        success: true,
        data: Some(event),
        error: None,
        message: Some("Event created".to_string()),
    }
    .print();
    Ok(())
}

pub async fn update_event(
    event_id: &str,
    calendar_id: Option<&str>,
    patch: EventPatch,
) -> AnyResult<()> {
    let event = update_event_record(event_id, calendar_id, patch).await?;
    Output {
        success: true,
        data: Some(event),
        error: None,
        message: Some(format!("Event {event_id} updated")),
    }
    .print();
    Ok(())
}

pub async fn delete_event(event_id: &str, calendar_id: Option<&str>) -> AnyResult<()> {
    delete_event_record(event_id, calendar_id).await?;
    Output::<()> {
        success: true,
        data: None,
        error: None,
        message: Some(format!("Event {event_id} deleted")),
    }
    .print();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    fn parse_day(value: &chrono::DateTime<chrono::Utc>) -> String {
        value.format("%Y-%m-%d").to_string()
    }

    #[test]
    fn test_build_event_supports_all_day_dates() {
        let event = build_event(EventInput {
            title: "All day".into(),
            start: "2026-04-03".into(),
            end: "2026-04-04".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(event.start.all_day);
        assert_eq!(event.start.value, "2026-04-03");
    }

    #[test]
    fn test_apply_event_patch_replaces_selected_fields() {
        let existing = build_event(EventInput {
            title: "Before".into(),
            start: "2026-04-03T09:00:00".into(),
            end: "2026-04-03T10:00:00".into(),
            description: Some("Old".into()),
            ..Default::default()
        })
        .unwrap();

        let updated = apply_event_patch(
            &existing,
            EventPatch {
                title: Some("After".into()),
                description: Some(String::new()),
                clear_reminders: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "After");
        assert!(updated.description.is_none());
        assert!(updated.reminders.is_empty());
    }

    #[test]
    fn test_apply_event_patch_can_clear_attendees() {
        let existing = CalendarEvent {
            attendees: vec![EventAttendee {
                email: "person@example.com".into(),
                ..Default::default()
            }],
            ..build_event(EventInput {
                title: "Before".into(),
                start: "2026-04-03T09:00:00".into(),
                end: "2026-04-03T10:00:00".into(),
                ..Default::default()
            })
            .unwrap()
        };

        let updated = apply_event_patch(
            &existing,
            EventPatch {
                attendees: Some(Vec::new()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.attendees.is_empty());
    }

    #[test]
    fn test_apply_event_patch_updates_timezone_without_replacing_times() {
        let existing = build_event(EventInput {
            title: "Before".into(),
            start: "2026-04-03T09:00:00".into(),
            end: "2026-04-03T10:00:00".into(),
            timezone: Some("America/New_York".into()),
            ..Default::default()
        })
        .unwrap();

        let updated = apply_event_patch(
            &existing,
            EventPatch {
                timezone: Some("America/Chicago".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.start.value, existing.start.value);
        assert_eq!(updated.end.value, existing.end.value);
        assert_eq!(updated.start.timezone.as_deref(), Some("America/Chicago"));
        assert_eq!(updated.end.timezone.as_deref(), Some("America/Chicago"));
    }

    #[test]
    fn test_apply_event_patch_ignores_timezone_only_change_for_all_day_events() {
        let existing = build_event(EventInput {
            title: "All day".into(),
            start: "2026-04-03".into(),
            end: "2026-04-04".into(),
            ..Default::default()
        })
        .unwrap();

        let updated = apply_event_patch(
            &existing,
            EventPatch {
                timezone: Some("America/Chicago".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.start.all_day);
        assert!(updated.end.all_day);
        assert!(updated.start.timezone.is_none());
        assert!(updated.end.timezone.is_none());
    }

    #[test]
    fn test_resolve_event_list_range_defaults_to_today() {
        let (start, end) = resolve_event_list_range(None, None, false).unwrap();
        let start = start.unwrap();
        let end = end.unwrap();

        assert!(end > start);
        assert!((end - start) <= chrono::Duration::hours(24));
    }

    #[test]
    fn test_resolve_event_list_range_supports_week_mode() {
        let (start, end) = resolve_event_list_range(None, None, true).unwrap();
        let start = start.unwrap();
        let end = end.unwrap();

        assert_eq!(start.weekday().num_days_from_monday(), 0);
        assert_eq!(end.weekday().num_days_from_monday(), 0);
    }

    #[test]
    fn test_resolve_event_list_range_supports_explicit_bounds() {
        let (start, end) =
            resolve_event_list_range(Some("2026-04-05"), Some("2026-04-06"), false).unwrap();
        let start = start.unwrap();
        let end = end.unwrap();

        assert_eq!(parse_day(&start), "2026-04-05");
        assert_eq!(parse_day(&end), "2026-04-07");
    }

    #[test]
    fn test_resolve_event_list_range_rejects_start_only() {
        let error = resolve_event_list_range(Some("2026-04-05"), None, false).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Config error: Explicit event ranges require both start and end."
        );
    }

    #[test]
    fn test_resolve_event_list_range_rejects_end_only() {
        let error = resolve_event_list_range(None, Some("2026-04-05"), false).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Config error: Explicit event ranges require both start and end."
        );
    }
}
