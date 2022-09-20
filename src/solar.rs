use chrono::{Date, DateTime, Datelike, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::location::Location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolarEvents {
    pub sunrise: DateTime<Utc>,
    pub sunset: DateTime<Utc>,
}

pub fn calculate_solar_events(
    location: &Location,
    date: &Date<Utc>,
) -> SolarEvents {
    let (sunrise, sunset) = sunrise::sunrise_sunset(
        location.absolute_latitude(),
        location.absolute_longitude(),
        date.year(),
        date.month(),
        date.day(),
    );

    let naive_sunrise = NaiveDateTime::from_timestamp(sunrise, 0);
    let naive_sunset = NaiveDateTime::from_timestamp(sunset, 0);

    SolarEvents{
        sunrise: DateTime::<Utc>::from_utc(naive_sunrise, Utc),
        sunset: DateTime::<Utc>::from_utc(naive_sunset, Utc),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Date, NaiveDate, Utc};

    use crate::location::Location;

    use super::calculate_solar_events;

    #[test]
    fn test_solar_events() {
        let location = Location::new(41.6, -71.5, "Narragansett Pier".into());
        let date = Date::<Utc>::from_utc(NaiveDate::from_ymd(2022, 07, 15), Utc);

        let sunrise_sunset = calculate_solar_events(&location, &date);
    }
}
