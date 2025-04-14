use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sunrise::{Coordinates, SolarDay, SolarEvent};

use crate::location::Location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolarEvents {
    pub sunrise: DateTime<Utc>,
    pub sunset: DateTime<Utc>,
}

impl From<&Location> for Option<Coordinates> {
    fn from(location: &Location) -> Self {
        Coordinates::new(location.relative_latitude(), location.relative_longitude())
    }
}

pub fn calculate_solar_events(location: &Location, date: &DateTime<Utc>) -> Option<SolarEvents> {
    let Some(coordinates) = location.into() else {
        return None;
    };

    let solar_day = SolarDay::new(coordinates, date.date_naive());

    let sunrise = solar_day.event_time(SolarEvent::Sunrise);
    let sunset = solar_day.event_time(SolarEvent::Sunset);

    Some(SolarEvents { sunrise, sunset })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

    use crate::location::Location;

    use super::calculate_solar_events;

    #[test]
    fn test_solar_events() {
        let location = Location::new(41.6, -71.5, "Narragansett Pier".into());
        let naive =
            NaiveDateTime::parse_from_str("2022-07-15 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let date = DateTime::from_naive_utc_and_offset(naive, Utc);

        let events = calculate_solar_events(&location, &date);
        assert!(events.is_some());

        let events = events.unwrap();
        let sunrise_date = NaiveDate::from_ymd_opt(2022, 7, 15).unwrap();

        // In July the sunset is late so in UTC it is on the next day
        let sunset_date = NaiveDate::from_ymd_opt(2022, 7, 16).unwrap();
        assert_eq!(events.sunrise.date_naive(), sunrise_date);
        assert_eq!(events.sunset.date_naive(), sunset_date);
    }
}
