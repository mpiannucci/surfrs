use std::collections::HashMap;

use chrono::Utc;
use chrono::prelude::*;
use csv::Reader;
use serde::Deserialize;
use serde::Serialize;

use crate::dimensional_data::DimensionalData;
use crate::swell::SwellSummary;
use crate::swell::{Swell, SwellProvider};
use crate::units::*;

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeteorologicalDataRecord {
    pub date: chrono::DateTime<Utc>,
    pub wind_direction: DimensionalData<Direction>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_gust_speed: DimensionalData<f64>,
    pub wave_height: DimensionalData<f64>,
    pub dominant_wave_period: DimensionalData<f64>,
    pub average_wave_period: DimensionalData<f64>,
    pub mean_wave_direction: DimensionalData<Direction>,
    pub air_pressure: DimensionalData<f64>,
    pub air_pressure_tendency: DimensionalData<f64>,
    pub air_temperature: DimensionalData<f64>,
    pub water_temperature: DimensionalData<f64>,
    pub dewpoint_temperature: DimensionalData<f64>,
    pub visibility: DimensionalData<f64>,
    pub tide: DimensionalData<f64>,
}

impl ParseableDataRecord for MeteorologicalDataRecord {
    type Metadata = ();

    fn from_data_row(
        _: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<MeteorologicalDataRecord, DataRecordParsingError> {
        let date = Utc
            .ymd(row[0].parse().unwrap(), row[1].parse().unwrap(), row[2].parse().unwrap())
            .and_hms(row[3].parse().unwrap(), row[4].parse().unwrap(), 0);

        Ok(MeteorologicalDataRecord {
            date,
            wind_direction: DimensionalData::from_raw_data(
                row[5],
                "wind direction".into(),
                Measurement::Direction,
                Units::Metric,
            ),
            wind_speed: DimensionalData::from_raw_data(
                row[6],
                "wind speed".into(),
                Measurement::Speed,
                Units::Metric,
            ),
            wind_gust_speed: DimensionalData::from_raw_data(
                row[7],
                "wind gust speed".into(),
                Measurement::Speed,
                Units::Metric,
            ),
            wave_height: DimensionalData::from_raw_data(
                row[8],
                "wave height".into(),
                Measurement::Length,
                Units::Metric,
            ),
            dominant_wave_period: DimensionalData::from_raw_data(
                row[9],
                "dominant wave period".into(),
                Measurement::Time,
                Units::Metric,
            ),
            average_wave_period: DimensionalData::from_raw_data(
                row[10],
                "average wave period".into(),
                Measurement::Time,
                Units::Metric,
            ),
            mean_wave_direction: DimensionalData::from_raw_data(
                row[11],
                "mean wave direction".into(),
                Measurement::Direction,
                Units::Metric,
            ),
            air_pressure: DimensionalData::from_raw_data(
                row[12],
                "air pressure".into(),
                Measurement::Pressure,
                Units::Metric,
            ),
            air_temperature: DimensionalData::from_raw_data(
                row[13],
                "air temperature".into(),
                Measurement::Temperature,
                Units::Metric,
            ),
            water_temperature: DimensionalData::from_raw_data(
                row[14],
                "water temperature".into(),
                Measurement::Temperature,
                Units::Metric,
            ),
            dewpoint_temperature: DimensionalData::from_raw_data(
                row[15],
                "dewpoint temperature".into(),
                Measurement::Temperature,
                Units::Metric,
            ),
            visibility: DimensionalData::from_raw_data(
                row[16],
                "".into(),
                Measurement::Visibility,
                Units::Metric,
            ),
            air_pressure_tendency: DimensionalData::from_raw_data(
                row[17],
                "air pressure tendency".into(),
                Measurement::Pressure,
                Units::Metric,
            ),
            tide: DimensionalData::from_raw_data(
                row[18],
                "tide".into(),
                Measurement::Length,
                Units::English,
            ),
        })
    }
}

impl UnitConvertible<MeteorologicalDataRecord> for MeteorologicalDataRecord {
    fn to_units(&mut self, new_units: &Units) {
        self.wind_direction.to_units(new_units);
        self.wind_speed.to_units(new_units);
        self.wind_gust_speed.to_units(new_units);
        self.wave_height.to_units(new_units);
        self.dominant_wave_period.to_units(new_units);
        self.average_wave_period.to_units(new_units);
        self.mean_wave_direction.to_units(new_units);
        self.air_pressure.to_units(new_units);
        self.air_pressure_tendency.to_units(new_units);
        self.air_temperature.to_units(new_units);
        self.water_temperature.to_units(new_units);
        self.dewpoint_temperature.to_units(new_units);
        self.visibility.to_units(new_units);
        self.tide.to_units(new_units);
    }
}

impl SwellProvider for MeteorologicalDataRecord {
    fn swell_data(&self) -> Result<SwellSummary, crate::swell::SwellProviderError> {
        Ok(SwellSummary {
            summary: Swell {
                wave_height: self.wave_height.clone(),
                period: self.dominant_wave_period.clone(),
                direction: self.mean_wave_direction.clone(),
                energy: None,
            }, 
            components: vec![],
        })
    }
}

impl From<MeteorologicalDataRecord> for HashMap<String, Option<String>> {
    fn from(m: MeteorologicalDataRecord) -> Self {
        HashMap::from([
            (m.wind_direction.variable_name.clone(), m.wind_direction.try_string()), 
            (m.wind_speed.variable_name.clone(), m.wind_speed.try_string()),
            (m.wind_gust_speed.variable_name.clone(), m.wind_gust_speed.try_string()),
            (m.wave_height.variable_name.clone(), m.wave_height.try_string()),
            (m.dominant_wave_period.variable_name.clone(), m.dominant_wave_period.try_string()),
            (m.average_wave_period.variable_name.clone(), m.average_wave_period.try_string()),
            (m.mean_wave_direction.variable_name.clone(), m.mean_wave_direction.try_string()),
            (m.air_pressure.variable_name.clone(), m.air_pressure.try_string()),
            (m.air_temperature.variable_name.clone(), m.air_temperature.try_string()),
            (m.water_temperature.variable_name.clone(), m.water_temperature.try_string()),
            (m.dewpoint_temperature.variable_name.clone(), m.dewpoint_temperature.try_string()),
            (m.visibility.variable_name.clone(), m.visibility.try_string()),
            (m.tide.variable_name.clone(), m.tide.try_string()),
        ])
        .into_iter()
        .filter(|v| v.1.is_some())
        .collect()
    }
}

pub struct MeteorologicalDataRecordCollection<'a> {
    reader: Reader<&'a [u8]>,
}

impl<'a> MeteorologicalDataRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        let reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(data.as_bytes());

        MeteorologicalDataRecordCollection { reader }
    }

    pub fn records(
        &'a mut self,
    ) -> impl Iterator<Item = MeteorologicalDataRecord> + 'a {
        self.reader.records().map(
            |result| -> Result<MeteorologicalDataRecord, DataRecordParsingError> {
                match result {
                    Ok(record) => {
                        let filtered_record: Vec<&str> =
                            record.iter().filter(|data| !data.is_empty()).collect();
                        let mut met_data =
                            MeteorologicalDataRecord::from_data_row(None, &filtered_record)?;
                        met_data.to_units(&Units::Metric);
                        Ok(met_data)
                    }
                    Err(e) => Err(DataRecordParsingError::ParseFailure(e.to_string())),
                }
            },
        )
        .filter_map(|d| d.ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_data_row_parse() {
        let raw_data = "2018 09 25 00 50  80 12.0 14.0   2.2     7   5.4 101 1032.4  16.5  19.4  12.9   MM +0.3    MM";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let met_data = MeteorologicalDataRecord::from_data_row(None, &data_row).unwrap();

        assert_eq!(met_data.date.year(), 2018);
        assert_eq!(met_data.wind_speed.value.unwrap(), 12.0);
        assert_eq!(met_data.wind_gust_speed.value.unwrap(), 14.0);
        assert!(met_data.tide.value.is_none());
    }
}
