use super::date_record::DateRecord;
use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};
use crate::dimensional_data::DimensionalData;
use crate::units::*;

use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct WaveDataRecord {
    pub date: DateRecord,
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

    fn from_data(raw_data: &str) -> Result<(Option<Self::Metadata>, Vec<WaveDataRecord>), DataRecordParsingError> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(raw_data.as_bytes());

        let records = reader
            .records()
            .map(|result| -> Result<WaveDataRecord, DataRecordParsingError> {
                if let Ok(record) = result {
                    let filtered_record: Vec<&str> =
                        record.iter().filter(|data| !data.is_empty()).collect();
                    let mut wave_data = WaveDataRecord::from_data_row(&None, &filtered_record)?;
                    wave_data.to_units(&Units::Metric);
                    return Ok(wave_data);
                }
                Err(DataRecordParsingError::InvalidData)
            })
            .collect();

        match records {
            Ok(records) => Ok((None, records)),
            Err(err) => Err(err)
        }
    }

    fn from_data_row(metadata: &Option<Self::Metadata>, row: &Vec<&str>) -> Result<WaveDataRecord, DataRecordParsingError> {
        Ok(WaveDataRecord {
            date: DateRecord::from_data_row(&None, row)?,
            wave_height: DimensionalData::from_raw_data(
                row[5],
                "wave height",
                Measurement::Length,
                Units::Metric,
            ),
            swell_wave_height: DimensionalData::from_raw_data(
                row[6],
                "swell wave height",
                Measurement::Length,
                Units::Metric,
            ),
            swell_wave_period: DimensionalData::from_raw_data(
                row[7],
                "swell period",
                Measurement::Time,
                Units::Metric,
            ),
            wind_wave_height: DimensionalData::from_raw_data(
                row[8],
                "wind wave height",
                Measurement::Length,
                Units::Metric,
            ),
            wind_wave_period: DimensionalData::from_raw_data(
                row[9],
                "wind period",
                Measurement::Time,
                Units::Metric,
            ),
            swell_wave_direction: DimensionalData::from_raw_data(
                row[10],
                "swell wave direction",
                Measurement::Direction,
                Units::Metric,
            ),
            wind_wave_direction: DimensionalData::from_raw_data(
                row[11],
                "wind wave direction",
                Measurement::Direction,
                Units::Metric,
            ),
            steepness: Steepness::from_str(row[12]).unwrap_or(Steepness::NA),
            average_wave_period: DimensionalData::from_raw_data(
                row[10],
                "average wave period",
                Measurement::Time,
                Units::Metric,
            ),
            mean_wave_direction: DimensionalData::from_raw_data(
                row[11],
                "mean wave direction",
                Measurement::Direction,
                Units::Metric,
            ),
        })
    }
}

impl UnitConvertible<WaveDataRecord> for WaveDataRecord {
    fn to_units(&mut self, new_units: &Units) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_data_row_parse() {
        let raw_data = "2018 09 25 00 43  2.0  0.4 12.5  1.9  6.2   E   E VERY_STEEP  5.0 101";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let wave_data = WaveDataRecord::from_data_row(&None, &data_row).unwrap();

        assert_eq!(wave_data.steepness, Steepness::VerySteep);
        assert_eq!(
            wave_data
                .swell_wave_direction
                .value
                .unwrap_or(Direction::from_degree(270))
                .direction,
            CardinalDirection::East
        );
        assert_eq!(
            wave_data
                .wind_wave_direction
                .value
                .unwrap_or(Direction::from_degree(270))
                .direction,
            CardinalDirection::East
        );
        assert!((wave_data.wave_height.value.unwrap_or(0.0) - 2.0).abs() < 0.0001);
        assert!((wave_data.swell_wave_height.value.unwrap_or(0.0) - 0.4).abs() < 0.0001);
    }
}
