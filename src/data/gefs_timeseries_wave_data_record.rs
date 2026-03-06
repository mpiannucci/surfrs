use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::dimensional_data::DimensionalData;
use crate::units::{Unit, UnitConvertible, UnitSystem};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GEFSTimeseriesWaveRecord {
    pub date: DateTime<Utc>,
    pub significant_wave_height_avg: DimensionalData<f64>,
    pub significant_wave_height_spread: DimensionalData<f64>,
    pub peak_period_avg: DimensionalData<f64>,
    pub peak_period_spread: DimensionalData<f64>,
    pub wind_speed_avg: DimensionalData<f64>,
    pub wind_speed_spread: DimensionalData<f64>,
}

impl ParseableDataRecord for GEFSTimeseriesWaveRecord {
    type Metadata = ();

    fn from_data_row(
        _metadata: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<Self, DataRecordParsingError>
    where
        Self: Sized,
    {
        if row.len() < 8 {
            return Err(DataRecordParsingError::ParseFailure(format!(
                "Expected 8 columns, got {}",
                row.len()
            )));
        }

        // row[0] = "20260305", row[1] = "12"
        let date_str = row[0];
        if date_str.len() != 8 {
            return Err(DataRecordParsingError::ParseFailure(
                "Expected 8-digit date YYYYMMDD".into(),
            ));
        }
        let year = date_str[0..4]
            .parse::<i32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse year".into()))?;
        let month = date_str[4..6]
            .parse::<u32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse month".into()))?;
        let day = date_str[6..8]
            .parse::<u32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse day".into()))?;
        let hour = row[1]
            .parse::<u32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse hour".into()))?;

        let date = Utc
            .with_ymd_and_hms(year, month, day, hour, 0, 0)
            .single()
            .ok_or_else(|| DataRecordParsingError::ParseFailure("Invalid date/time".into()))?;

        Ok(GEFSTimeseriesWaveRecord {
            date,
            significant_wave_height_avg: DimensionalData::from_raw_data(
                row[2],
                "significant wave height avg".into(),
                Unit::Meters,
            ),
            significant_wave_height_spread: DimensionalData::from_raw_data(
                row[3],
                "significant wave height spread".into(),
                Unit::Meters,
            ),
            peak_period_avg: DimensionalData::from_raw_data(
                row[4],
                "peak period avg".into(),
                Unit::Seconds,
            ),
            peak_period_spread: DimensionalData::from_raw_data(
                row[5],
                "peak period spread".into(),
                Unit::Seconds,
            ),
            wind_speed_avg: DimensionalData::from_raw_data(
                row[6],
                "wind speed avg".into(),
                Unit::MetersPerSecond,
            ),
            wind_speed_spread: DimensionalData::from_raw_data(
                row[7],
                "wind speed spread".into(),
                Unit::MetersPerSecond,
            ),
        })
    }
}

impl UnitConvertible for GEFSTimeseriesWaveRecord {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.significant_wave_height_avg.to_units(new_units);
        self.significant_wave_height_spread.to_units(new_units);
        self.wind_speed_avg.to_units(new_units);
        self.wind_speed_spread.to_units(new_units);
        self
    }
}

pub struct GEFSTimeseriesWaveRecordCollection<'a> {
    data: &'a str,
}

impl<'a> GEFSTimeseriesWaveRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        GEFSTimeseriesWaveRecordCollection { data }
    }

    pub fn records(&'a self) -> impl Iterator<Item = GEFSTimeseriesWaveRecord> + 'a {
        self.data
            .lines()
            .filter(|line| is_data_row(line))
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                GEFSTimeseriesWaveRecord::from_data_row(None, &cols).ok()
            })
    }
}

/// Returns true for lines that start with an 8-digit date (YYYYMMDD).
fn is_data_row(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or("");
    first.len() == 8 && first.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    const SAMPLE: &str = " date   hour Hs avg Hs spr Tp avg Tp spr U10avg U10spr \n               (m)    (m)    (s)    (s)  (m/s)  (m/s)  \n ----------------------------------------------------- \n 20260305 12  1.05   0.03   7.14   0.25   1.16   0.56 \n 20260305 15  1.01   0.03   7.61   0.93   1.17   0.66 \n 20260306 00  1.10   0.06   9.03   0.92   8.71   0.93 \n";

    #[test]
    fn test_parse_gefs_timeseries() {
        let collection = GEFSTimeseriesWaveRecordCollection::from_data(SAMPLE);
        let rows: Vec<_> = collection.records().collect();

        assert_eq!(rows.len(), 3);

        let first = &rows[0];
        assert_eq!(first.date.year(), 2026);
        assert_eq!(first.date.month(), 3);
        assert_eq!(first.date.day(), 5);
        assert_eq!(first.date.hour(), 12);
        assert!((first.significant_wave_height_avg.value.unwrap() - 1.05).abs() < 0.001);
        assert!((first.significant_wave_height_spread.value.unwrap() - 0.03).abs() < 0.001);
        assert!((first.peak_period_avg.value.unwrap() - 7.14).abs() < 0.001);
        assert!((first.peak_period_spread.value.unwrap() - 0.25).abs() < 0.001);
        assert!((first.wind_speed_avg.value.unwrap() - 1.16).abs() < 0.001);
        assert!((first.wind_speed_spread.value.unwrap() - 0.56).abs() < 0.001);

        let third = &rows[2];
        assert_eq!(third.date.day(), 6);
        assert_eq!(third.date.hour(), 0);
    }
}
