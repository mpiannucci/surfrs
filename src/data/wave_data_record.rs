use chrono::{DateTime, TimeZone, Utc};
use csv::Reader;
use serde::{Deserialize, Serialize};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};
use crate::dimensional_data::DimensionalData;
use crate::swell::{Swell, SwellProvider, SwellSummary};
use crate::units::*;

use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaveDataRecord {
    pub date: DateTime<Utc>,
    pub wave_height: DimensionalData<f64>,
    pub swell_wave_height: DimensionalData<f64>,
    pub swell_wave_period: DimensionalData<f64>,
    pub wind_wave_height: DimensionalData<f64>,
    pub wind_wave_period: DimensionalData<f64>,
    pub swell_wave_direction: DimensionalData<Direction>,
    pub wind_wave_direction: DimensionalData<Direction>,
    pub steepness: Steepness,
    pub average_wave_period: DimensionalData<f64>,
    pub mean_wave_direction: DimensionalData<Direction>,
}

impl ParseableDataRecord for WaveDataRecord {
    type Metadata = ();

    fn from_data_row(
        _: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<WaveDataRecord, DataRecordParsingError> {
        let date = Utc
            .with_ymd_and_hms(
                row[0].parse().unwrap(),
                row[1].parse().unwrap(),
                row[2].parse().unwrap(),
                row[3].parse().unwrap(),
                row[4].parse().unwrap(),
                0,
            )
            .unwrap();

        Ok(WaveDataRecord {
            date,
            wave_height: DimensionalData::from_raw_data(
                row[5],
                "wave height".into(),
                Unit::Meters,
            ),
            swell_wave_height: DimensionalData::from_raw_data(
                row[6],
                "swell wave height".into(),
                Unit::Meters
            ),
            swell_wave_period: DimensionalData::from_raw_data(
                row[7],
                "swell period".into(),
                Unit::Seconds
            ),
            wind_wave_height: DimensionalData::from_raw_data(
                row[8],
                "wind wave height".into(),
                Unit::Meters,
            ),
            wind_wave_period: DimensionalData::from_raw_data(
                row[9],
                "wind period".into(),
                Unit::Seconds,
            ),
            swell_wave_direction: DimensionalData::from_raw_data(
                row[10],
                "swell wave direction".into(), 
                Unit::Degrees,
            ),
            wind_wave_direction: DimensionalData::from_raw_data(
                row[11],
                "wind wave direction".into(),
                Unit::Degrees,
            ),
            steepness: Steepness::from_str(row[12]).unwrap_or(Steepness::NA),
            average_wave_period: DimensionalData::from_raw_data(
                row[10],
                "average wave period".into(),
                Unit::Seconds,
            ),
            mean_wave_direction: DimensionalData::from_raw_data(
                row[11],
                "mean wave direction".into(),
                Unit::Degrees,
            ),
        })
    }
}

impl UnitConvertible<WaveDataRecord> for WaveDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) {
        self.wave_height.to_units(new_units);
        self.average_wave_period.to_units(new_units);
        self.mean_wave_direction.to_units(new_units);
        self.swell_wave_height.to_units(new_units);
        self.swell_wave_period.to_units(new_units);
        self.swell_wave_direction.to_units(new_units);
        self.wind_wave_height.to_units(new_units);
        self.wind_wave_period.to_units(new_units);
        self.wind_wave_direction.to_units(new_units);
    }
}

impl SwellProvider for WaveDataRecord {
    fn swell_data(&self) -> Result<SwellSummary, crate::swell::SwellProviderError> {
        Ok(SwellSummary {
            summary: Swell {
                wave_height: self.wave_height.clone(),
                period: self.average_wave_period.clone(),
                direction: self.mean_wave_direction.clone(),
                energy: None,
            },
            components: vec![
                Swell {
                    wave_height: self.swell_wave_height.clone(),
                    period: self.swell_wave_period.clone(),
                    direction: self.swell_wave_direction.clone(),
                    energy: None,
                },
                Swell {
                    wave_height: self.wind_wave_height.clone(),
                    period: self.wind_wave_period.clone(),
                    direction: self.wind_wave_direction.clone(),
                    energy: None,
                },
            ],
        })
    }
}

pub struct WaveDataRecordCollection<'a> {
    reader: Reader<&'a [u8]>,
}

impl<'a> WaveDataRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        let reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(data.as_bytes());

        WaveDataRecordCollection { reader }
    }

    pub fn records(&'a mut self) -> impl Iterator<Item = WaveDataRecord> + 'a {
        self.reader
            .records()
            .map(|result| -> Result<WaveDataRecord, DataRecordParsingError> {
                if let Ok(record) = result {
                    let filtered_record: Vec<&str> =
                        record.iter().filter(|data| !data.is_empty()).collect();
                    let mut wave_data = WaveDataRecord::from_data_row(None, &filtered_record)?;
                    wave_data.to_units(&UnitSystem::Metric);
                    return Ok(wave_data);
                }
                Err(DataRecordParsingError::InvalidData)
            })
            .filter_map(|d| d.ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_data_row_parse() {
        let raw_data = "2018 09 25 00 43  2.0  0.4 12.5  1.9  6.2   E   E VERY_STEEP  5.0 101";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let wave_data = WaveDataRecord::from_data_row(None, &data_row).unwrap();

        assert_eq!(wave_data.steepness, Steepness::VerySteep);
        assert_eq!(
            wave_data
                .swell_wave_direction
                .value
                .unwrap_or(Direction::from_degrees(270))
                .cardinal_direction()
                .clone(),
            CardinalDirection::East
        );
        assert_eq!(
            wave_data
                .wind_wave_direction
                .value
                .unwrap_or(Direction::from_degrees(270))
                .cardinal_direction()
                .clone(),
            CardinalDirection::East
        );
        assert!((wave_data.wave_height.value.unwrap_or(0.0) - 2.0).abs() < 0.0001);
        assert!((wave_data.swell_wave_height.value.unwrap_or(0.0) - 0.4).abs() < 0.0001);
    }
}
