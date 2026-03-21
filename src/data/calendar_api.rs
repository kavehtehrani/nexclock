use serde::{Deserialize, Serialize};
use tracing::error;

use crate::constants::ANYCALENDAR_API_BASE;
use crate::error::NexClockError;

/// A single calendar date result returned to the UI.
#[derive(Debug, Clone)]
pub struct CalendarDateEntry {
    pub calendar_id: String,
    pub display: String,
    pub native_display: String,
}

// ── API response types (private) ─────────────────────────────────────

#[derive(Deserialize)]
struct NowResponse {
    results: Vec<NowResult>,
}

#[derive(Deserialize)]
struct NowResult {
    date: DateInfo,
}

#[derive(Deserialize)]
struct DateInfo {
    calendar: String,
    display: String,
    native_display: String,
}

// ── Fetch functions ──────────────────────────────────────────────────

/// Fetches today's date in a single calendar system from the AnyCalendar API.
pub async fn fetch_calendar_date(
    calendar_id: &str,
    timezone: &str,
) -> Result<CalendarDateEntry, NexClockError> {
    let url = format!(
        "{ANYCALENDAR_API_BASE}/now/{calendar_id}?timezone={timezone}"
    );
    let response = reqwest::get(&url).await?;
    let data: NowResponse = response.json().await?;

    let result = data
        .results
        .into_iter()
        .next()
        .ok_or_else(|| NexClockError::Parse("empty results from AnyCalendar API".into()))?;

    Ok(CalendarDateEntry {
        calendar_id: result.date.calendar,
        display: result.date.display,
        native_display: result.date.native_display,
    })
}

/// Fetches today's date in multiple calendar systems.
/// Tolerates partial failures: logs errors and returns only successful results.
pub async fn fetch_all_calendar_dates(
    calendar_ids: &[String],
    timezone: &str,
) -> Vec<CalendarDateEntry> {
    let mut results = Vec::with_capacity(calendar_ids.len());
    for id in calendar_ids {
        match fetch_calendar_date(id, timezone).await {
            Ok(entry) => results.push(entry),
            Err(err) => error!("Calendar fetch failed for '{id}': {err}"),
        }
    }
    results
}

// ── Month data (for the calendar grid component) ─────────────────────

/// Full month grid data for rendering a calendar component.
#[derive(Debug, Clone)]
pub struct MonthData {
    pub calendar: String,
    pub year: i64,
    pub month: u32,
    pub month_name: String,
    pub days_in_month: u32,
    pub first_weekday: u32, // 0=Mon..6=Sun (matches chrono)
    pub today: Option<u32>, // day number of "today" if this is the current month
}

#[derive(Deserialize)]
struct MonthResponse {
    year: i64,
    month: u32,
    month_name: String,
    days_in_month: u32,
    first_weekday: u32,
    #[allow(dead_code)]
    days: Vec<MonthDay>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MonthDay {
    day: u32,
}

#[derive(Serialize)]
struct MonthRequest {
    calendar: String,
}

/// Fetches the current month grid for a calendar system.
/// Also determines which day is "today" by calling /now.
pub async fn fetch_month(
    calendar_id: &str,
    timezone: &str,
) -> Result<MonthData, NexClockError> {
    let client = reqwest::Client::new();

    // Fetch current month grid
    let month_url = format!("{ANYCALENDAR_API_BASE}/month");
    let body = MonthRequest {
        calendar: calendar_id.to_string(),
    };
    let month_resp: MonthResponse = client
        .post(&month_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    // Fetch today's date to know which day to highlight
    let now_entry = fetch_calendar_date(calendar_id, timezone).await.ok();
    let today = now_entry.and_then(|entry| {
        // Parse day from display string (format: "29 Esfand 1404")
        // The first token is always the day number
        entry.display.split_whitespace().next()?.parse::<u32>().ok()
    });

    Ok(MonthData {
        calendar: calendar_id.to_string(),
        year: month_resp.year,
        month: month_resp.month,
        month_name: month_resp.month_name,
        days_in_month: month_resp.days_in_month,
        first_weekday: month_resp.first_weekday,
        today,
    })
}
