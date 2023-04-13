use chrono::{DateTime, Duration, Timelike, Utc};

/// Creates a datetime object for the most recent model run output for stations data given the logic that
/// weather models run at 0Z, 6Z, 12Z, and 18Z
pub fn closest_gfs_model_stations_datetime(datetime: &DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = *datetime + Duration::hours(-6);
    let latest_model_hour = adjusted.hour() % 6;
    (adjusted - Duration::hours(latest_model_hour as i64))
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

/// Creates a datetime object for the most recent model run for gridded data given the logic that
/// weather models run at 0Z, 6Z, 12Z, and 18Z
pub fn closest_gfs_model_gridded_datetime(datetime: &DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = *datetime + Duration::hours(-5);
    let latest_model_hour = adjusted.hour() % 6;
    (adjusted - Duration::hours(latest_model_hour as i64))
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

/// Creates a datetime object for the most recent model run given the logic that
/// weather models run every hour
pub fn closest_hourly_model_datetime(datetime: &DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = *datetime + Duration::hours(-2);
    adjusted
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

#[cfg(test)]
mod test {
    use crate::tools::date::{closest_gfs_model_gridded_datetime, closest_gfs_model_stations_datetime};

    #[test]
    fn test_closest_station_model_datetime() {
        use chrono::prelude::*; 

        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 12, 0, 0).unwrap();

        let result = closest_gfs_model_stations_datetime(&datetime);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_closest_gridded_model_datetime() {
        use chrono::prelude::*; 

        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 18, 0, 0).unwrap();

        let result = closest_gfs_model_gridded_datetime(&datetime);
        assert_eq!(result, expected);
    }
}