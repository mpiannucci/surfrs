use chrono::{DateTime, Duration, Timelike, Utc};

// Rounds the datetime to the nearest multiple hour
// For example, if the datetime is 2023-04-13 23:00:00 and the multiplier is 6, the result would be 2023-04-13 18:00:00
// If the datetime is 2023-04-13 23:00:00 and the multiplier is 3, the result would be 2023-04-13 21:00:00
// This operation is a floor, meaning that it will only round down to the nearest multiple hour
pub fn round_to_nearest_multiple_hour(datetime: &DateTime<Utc>, multiplier: u32) -> DateTime<Utc> {
    let hour = datetime.hour();
    let remainder = hour % multiplier;
    let rounded_hour = hour - remainder;
    datetime
        .with_hour(rounded_hour)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

/// Creates a datetime object for the most recent model run output for stations data given the logic that
/// weather models run at 0Z, 6Z, 12Z, and 18Z
pub fn closest_gfs_model_stations_datetime(datetime: &DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = *datetime + Duration::hours(-6);
    round_to_nearest_multiple_hour(&adjusted, 6)
}

/// Creates a datetime object for the most recent model run for gridded data given the logic that
/// weather models run at 0Z, 6Z, 12Z, and 18Z
pub fn closest_gfs_model_gridded_datetime(datetime: &DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = *datetime + Duration::hours(-5);
    round_to_nearest_multiple_hour(&adjusted, 6)
}

// Creates a list of datetimes corresponding to all of the model runs for the GFS model within 
// the given start and end datetimes. 
// 
// This function does not account for the fact that the most 
// recent model run may not have been completed yet. To filter out any uncompleted model runs,
// use the closest_gfs_model_stations_datetime or closest_gfs_model_gridded_datetime functions to get
// the most recent model run and then filter out any datetimes that are greater than the most recent model run
pub fn gfs_model_fmrc_datetimes(
    (start, end): (DateTime<Utc>, DateTime<Utc>),
) -> Vec<DateTime<Utc>> {
    let mut datetimes = vec![];
    let mut current = round_to_nearest_multiple_hour(&start, 6);
    while current <= end {
        datetimes.push(current);
        current = current + Duration::hours(6);
    }
    datetimes
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
    use crate::tools::date::{
        closest_gfs_model_gridded_datetime, closest_gfs_model_stations_datetime,
        closest_hourly_model_datetime, gfs_model_fmrc_datetimes, round_to_nearest_multiple_hour,
    };
    use chrono::prelude::*;

    #[test]
    fn test_round_to_nearest_multiple_hour() {
        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 18, 0, 0).unwrap();
        let result = round_to_nearest_multiple_hour(&datetime, 6);
        assert_eq!(result, expected);

        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 21, 0, 0).unwrap();
        let result = round_to_nearest_multiple_hour(&datetime, 3);
        assert_eq!(result, expected);

        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let result = round_to_nearest_multiple_hour(&datetime, 1);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_gfs_closest_station_model_datetime() {
        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 12, 0, 0).unwrap();

        let result = closest_gfs_model_stations_datetime(&datetime);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_gfs_closest_gridded_model_datetime() {
        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 18, 0, 0).unwrap();

        let result = closest_gfs_model_gridded_datetime(&datetime);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_gfs_gridded_model_fmrc_datetime() {
        let start = Utc.with_ymd_and_hms(2023, 4, 13, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2023, 4, 14, 0, 0, 0).unwrap();

        let result = gfs_model_fmrc_datetimes((start, end));
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], start);
        assert_eq!(result[4], end);
    }

    #[test]
    fn test_closest_hourly_model_datetime() {
        let datetime = Utc.with_ymd_and_hms(2023, 4, 13, 23, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2023, 4, 13, 21, 0, 0).unwrap();

        let result = closest_hourly_model_datetime(&datetime);
        assert_eq!(result, expected);
    }
}
