use anyhow::Result as AnyResult;

use crate::caldav::CalDavClient;
use crate::config::Config;
use crate::models::Output;

fn calendar_client() -> crate::error::Result<CalDavClient> {
    let config = Config::load()?;
    let username = config.get_username()?;
    let app_password = config.get_app_password()?;
    Ok(CalDavClient::new(username, app_password))
}

pub async fn list_calendars() -> AnyResult<()> {
    let client = calendar_client()?;
    let calendars = client.list_calendars().await?;
    Output::success(calendars).print();
    Ok(())
}

pub async fn create_calendar_record(
    name: &str,
    color: Option<&str>,
) -> crate::error::Result<crate::caldav::Calendar> {
    let client = calendar_client()?;
    client.create_calendar(name, color).await
}

pub async fn update_calendar_record(
    calendar_id: &str,
    name: Option<&str>,
    color: Option<&str>,
) -> crate::error::Result<crate::caldav::Calendar> {
    let client = calendar_client()?;
    let existing = client.get_calendar_by_id(calendar_id).await?;
    client.update_calendar(&existing, name, color).await
}

pub async fn delete_calendar_record(calendar_id: &str) -> crate::error::Result<()> {
    let client = calendar_client()?;
    let existing = client.get_calendar_by_id(calendar_id).await?;
    client.delete_calendar(&existing).await
}

pub async fn create_calendar(name: &str, color: Option<&str>) -> AnyResult<()> {
    let calendar = create_calendar_record(name, color).await?;
    Output {
        success: true,
        data: Some(calendar),
        error: None,
        message: Some("Calendar created".to_string()),
    }
    .print();
    Ok(())
}

pub async fn update_calendar(
    calendar_id: &str,
    name: Option<&str>,
    color: Option<&str>,
) -> AnyResult<()> {
    let calendar = update_calendar_record(calendar_id, name, color).await?;
    Output {
        success: true,
        data: Some(calendar),
        error: None,
        message: Some(format!("Calendar {calendar_id} updated")),
    }
    .print();
    Ok(())
}

pub async fn delete_calendar(calendar_id: &str) -> AnyResult<()> {
    delete_calendar_record(calendar_id).await?;
    Output::<()> {
        success: true,
        data: None,
        error: None,
        message: Some(format!("Calendar {calendar_id} deleted")),
    }
    .print();
    Ok(())
}
