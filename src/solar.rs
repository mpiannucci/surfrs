use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::location::Location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolarEvents {
    pub sunrise: DateTime<Utc>,
    pub sunset: DateTime<Utc>,
}

pub fn calculate_solar_events(
    location: &Location,
    date: &DateTime<Utc>,
) -> SolarEvents {
    let (sunrise, sunset) = sunrise::sunrise_sunset(
        location.absolute_latitude(),
        location.absolute_longitude(),
        date.year(),
        date.month(),
        date.day(),
    );

    let sunrise = DateTime::from_timestamp(sunrise, 0).unwrap();
    let sunset = DateTime::from_timestamp(sunset, 0).unwrap();

    SolarEvents{
        sunrise,
        sunset,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDateTime, Utc};

    use crate::location::Location;

    use super::calculate_solar_events;

    #[test]
    fn test_solar_events() {
        let location = Location::new(41.6, -71.5, "Narragansett Pier".into());
        let naive = NaiveDateTime::parse_from_str("2022-07-15 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let date = DateTime::from_naive_utc_and_offset(naive, Utc);
        let _ = calculate_solar_events(&location, &date);
    }
}
