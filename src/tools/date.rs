use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

/// Creates a datetime object for the most recent model run given the logic that 
/// weather models run at 0Z, 6Z, 12Z, and 18Z
pub fn closest_model_datetime(datetime: DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = datetime + Duration::hours(-6);
    let latest_model_hour = adjusted.hour() % 6;
    (adjusted - Duration::hours(latest_model_hour as i64))
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}