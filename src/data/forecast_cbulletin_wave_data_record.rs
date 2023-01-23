use std::str::FromStr;

use chrono::{DateTime, Utc, Datelike, TimeZone};
use csv::Reader;
use regex::Regex;
use serde::{Serialize, Deserialize};

use crate::dimensional_data::DimensionalData;
use crate::location::Location;
use crate::swell::{Swell, SwellProvider, SwellSummary};
use crate::units::{Direction, UnitConvertible, Unit, UnitSystem};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForecastCBulletinWaveRecordMetadata {
    pub location: Location,
    pub model_run_date: DateTime<Utc>,
}

impl FromStr for ForecastCBulletinWaveRecordMetadata {
    type Err = DataRecordParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();

        let location_parser = Regex::new("Location\\s*:\\s*(.{0,10})\\s*\\(([+-]?[0-9]*[.]?[0-9]+[N|S])\\s*([+-]?[0-9]*[.]?[0-9]+[E|W])\\)");
        let location_parser = location_parser.map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to create location regex: {}", e))
        })?;

        let location_str = lines.next().ok_or(DataRecordParsingError::ParseFailure(
            "Invalid data for location metadata".into(),
        ))?;
        let location = match location_parser.captures(location_str) {
            Some(captures) => {
                let name = captures.get(1).unwrap().as_str().trim();
                let latitude_str = captures.get(2).unwrap().as_str();
                let longitude_str = captures.get(3).unwrap().as_str();

                let latitude = parse_latitude(latitude_str)?;
                let longitude = parse_longitude(longitude_str)?;

                Ok(Location::new(latitude, longitude, name.into()))
            }
            None => Err(DataRecordParsingError::ParseFailure(
                "Failed to capture location data from regex".into(),
            )),
        }?;

        // Skip the second line
        lines.next();

        // The third has the model run date and time
        let model_run_parser =
            Regex::new("Cycle\\s*:\\s*([0-9]{0,4})([0-9]{0,2})([0-9]{0,2})\\s*([0-9]{0,2})");
        let model_run_parser = model_run_parser.map_err(|_| {
            DataRecordParsingError::ParseFailure(
                "Failed to create regex to parse model date run".into(),
            )
        })?;

        let model_run_date = match model_run_parser.captures(lines.next().unwrap_or("")) {
            Some(captures) => {
                let year = captures
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse::<i32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure(
                            "Failed to capture model date year".into(),
                        )
                    })?;
                let month = captures
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure(
                            "Failed to capture model date month".into(),
                        )
                    })?;
                let day = captures
                    .get(3)
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure(
                            "Failed to capture model date day".into(),
                        )
                    })?;
                let hour = captures
                    .get(4)
                    .unwrap()
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure(
                            "Failed to capture model date hour".into(),
                        )
                    })?;
                let minute = 0;

                let d = Utc.with_ymd_and_hms(year, month, day, hour, minute, 0).unwrap();
                Ok(d)
            }
            None => Err(DataRecordParsingError::ParseFailure(
                "Failed to capture model run date from regex".into(),
            )),
        }?;

        Ok(ForecastCBulletinWaveRecordMetadata {
            location,
            model_run_date,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForecastCBulletinWaveRecord {
    pub date: DateTime<Utc>,
    pub significant_wave_height: DimensionalData<f64>,
    pub swell_components: Vec<Swell>,
}

impl ParseableDataRecord for ForecastCBulletinWaveRecord {
    type Metadata = ForecastCBulletinWaveRecordMetadata;

    fn from_data_row(
        metadata: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<Self, DataRecordParsingError>
    where
        Self: Sized,
    {
        let timestep = row[0];
        let day = timestep[0..2].parse::<u32>().map_err(|_| {
            DataRecordParsingError::ParseFailure("Failed to parse day from timestep".into())
        })?;
        let hour = timestep[2..].parse::<u32>().map_err(|_| {
            DataRecordParsingError::ParseFailure("Failed to parse hour from timestep".into())
        })?;

        let model_date = match metadata {
            Some(m) => Ok(m.model_run_date.date_naive()), 
            None => Err(DataRecordParsingError::InvalidData),
        }?;

        let month = if model_date.day() > day {
            model_date.month() + 1
        } else {
            model_date.month()
        };

        let date = Utc.with_ymd_and_hms(model_date.year(), month, day, hour, 0, 0).unwrap();

        let significant_wave_height = DimensionalData::from_raw_data(
            row[1],
            "significant wave height".into(),
            Unit::Feet,
        );

        let mut swell_components = Vec::new();

        for i in (2..row.len()).step_by(3) {
            let wave_height = row[i].parse::<f64>().map_err(|_| {
                DataRecordParsingError::ParseFailure("Failed to parse height from row".into())
            })?;
            let period = row[i + 1].parse::<f64>().map_err(|_| {
                DataRecordParsingError::ParseFailure("Failed to parse period from row".into())
            })?;
            let degrees = row[i + 2].parse::<i32>().map_err(|_| {
                DataRecordParsingError::ParseFailure("Failed to parse direction from row".into())
            })?;

            swell_components.push(Swell::new(
                &UnitSystem::English,
                wave_height,
                period,
                Direction::from_degrees(degrees),
                None,
            ));
        }

        Ok(ForecastCBulletinWaveRecord {
            date,
            significant_wave_height,
            swell_components,
        })
    }
}

impl UnitConvertible<ForecastCBulletinWaveRecord> for ForecastCBulletinWaveRecord {
    fn to_units(&mut self, new_units: &UnitSystem) {
        self.significant_wave_height.to_units(new_units);
        for swell in &mut self.swell_components {
            swell.to_units(new_units);
        }
    }
}

impl SwellProvider for ForecastCBulletinWaveRecord {
    fn swell_data(&self) -> Result<SwellSummary, crate::swell::SwellProviderError> {
        let dominant = self.swell_components[0].clone();
        Ok(SwellSummary {
            summary: Swell {
                wave_height: self.significant_wave_height.clone(),
                period: dominant.period, 
                direction: dominant.direction,
                energy: None,
            },
            components: self.swell_components.clone(),
        })
    }
}

fn parse_latitude(raw: &str) -> Result<f64, DataRecordParsingError> {
    let latitude = raw[0..raw.len() - 1].parse::<f64>().map_err(|e| {
        DataRecordParsingError::ParseFailure(format!("Failed to parse latitude: {:?}", e))
    })?;

    if raw.contains('S') {
        Ok(-latitude)
    } else {
        Ok(latitude)
    }
}

fn parse_longitude(raw: &str) -> Result<f64, DataRecordParsingError> {
    let longitude = raw[0..raw.len() - 1].parse::<f64>().map_err(|e| {
        DataRecordParsingError::ParseFailure(format!("Failed to parse longitude: {:?}", e))
    })?;

    if raw.contains('W') {
        Ok(-longitude)
    } else {
        Ok(longitude)
    }
}

pub struct ForecastCBulletinWaveRecordCollection<'a> {
    data: &'a str,
    reader: Reader<&'a [u8]>,
}

impl<'a> ForecastCBulletinWaveRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        let reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(data.as_bytes());

        ForecastCBulletinWaveRecordCollection { data, reader }
    }

    pub fn records(
        &'a mut self,
    ) -> Result<
        (
            ForecastCBulletinWaveRecordMetadata,
            impl Iterator<Item = ForecastCBulletinWaveRecord> + 'a,
        ),
        DataRecordParsingError,
    > {
        let metadata = self.data.parse::<ForecastCBulletinWaveRecordMetadata>()?;
        let metadata_clone = metadata.clone();
        let records = self
            .reader
            .records()
            .skip(5)
            .map(
                move |result| -> Result<ForecastCBulletinWaveRecord, DataRecordParsingError> {
                    match result {
                        Ok(record) => {
                            let filtered_record: Vec<&str> =
                                record.iter().filter(|data| !data.is_empty()).collect();
                            let mut wave_data = ForecastCBulletinWaveRecord::from_data_row(
                                Some(&metadata),
                                &filtered_record,
                            )?;
                            wave_data.to_units(&UnitSystem::Metric);
                            Ok(wave_data)
                        }
                        Err(e) => Err(DataRecordParsingError::ParseFailure(format!(
                            "Failed to parse record: {}",
                            e
                        ))),
                    }
                },
            )
            .filter_map(|d| d.ok());

        Ok((metadata_clone, records))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[test]
    fn test_parse_latitude() {
        assert_eq!(parse_latitude("12.3456N").unwrap(), 12.3456);
        assert_eq!(parse_latitude("12.3456S").unwrap(), -12.3456);
    }

    #[test]
    fn longitude() {
        assert_eq!(parse_longitude("12.3456E").unwrap(), 12.3456);
        assert_eq!(parse_longitude("12.3456W").unwrap(), -12.3456);
    }

    #[test]
    fn parse_wave_cbulletin_metadata() {
        let metadata = "Location : 44097      (40.98N  71.12W)
        Model    : spectral resolution for points
        Cycle    : 20220519 18 UTC
        ";

        let metadata = ForecastCBulletinWaveRecordMetadata::from_str(metadata).unwrap();
        assert_eq!(metadata.location.name, "44097");
        assert_eq!(metadata.location.latitude, 40.98);
        assert_eq!(metadata.location.longitude, -71.12);

        assert_eq!(metadata.model_run_date.year(), 2022);
        assert_eq!(metadata.model_run_date.month(), 5);
        assert_eq!(metadata.model_run_date.day(), 19);
        assert_eq!(metadata.model_run_date.hour(), 18);
    }

    #[test]
    fn test_wave_cbulletin_row_parse() {
        let metadata = ForecastCBulletinWaveRecordMetadata {
            location: Location::new(40.98, -71.12, "".into()),
            model_run_date: Utc.with_ymd_and_hms(2020, 05, 19, 18, 0, 0).unwrap()
        };

        let row = "0118  3  2 04 142  2 07 163                                        ";
        let row = row.split_whitespace().collect();

        let wave_bulletin_record =
            ForecastCBulletinWaveRecord::from_data_row(Some(&metadata), &row).unwrap();

        assert_eq!(wave_bulletin_record.date.year(), 2020);
        assert_eq!(wave_bulletin_record.date.month(), 6);
        assert_eq!(wave_bulletin_record.date.day(), 01);
        assert_eq!(wave_bulletin_record.date.hour(), 18);
        assert!((wave_bulletin_record.significant_wave_height.value.unwrap() - 3.0).abs() < 0.01);
        assert_eq!(wave_bulletin_record.swell_components.len(), 2);
        assert_eq!(
            wave_bulletin_record.swell_components[0]
                .wave_height
                .value
                .unwrap(),
            2.0
        );
        assert_eq!(
            wave_bulletin_record.swell_components[0]
                .period
                .value
                .unwrap(),
            4.0
        );
        assert_eq!(
            wave_bulletin_record.swell_components[0]
                .direction
                .value
                .as_ref()
                .unwrap()
                .degrees,
            142
        );
        assert_eq!(
            wave_bulletin_record.swell_components[1]
                .wave_height
                .value
                .unwrap(),
            2.0
        );
        assert_eq!(
            wave_bulletin_record.swell_components[1]
                .period
                .value
                .unwrap(),
            7.0
        );
        assert_eq!(
            wave_bulletin_record.swell_components[1]
                .direction
                .value
                .as_ref()
                .unwrap()
                .degrees,
            163
        );
    }
}
