use std::str::FromStr;

use regex::Regex;

use crate::dimensional_data::DimensionalData;
use crate::units::{Direction};
use crate::swell::{Swell};
use crate::location::{Location};

use super::date_record::DateRecord;
use super::parseable_data_record::{ParseableDataRecord, DataRecordParsingError};

#[derive(Clone, Debug)]
pub struct ForecastBulletinWaveRecordMetadata {
    pub location: Location,
    pub model_run_date: DateRecord,
}

impl FromStr for ForecastBulletinWaveRecordMetadata {
    type Err = DataRecordParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();

        let location_parser = Regex::new("Location\\s*:\\s*(.{0,10})\\s*\\(([+-]?[0-9]*[.]?[0-9]+[N|S])\\s*([+-]?[0-9]*[.]?[0-9]+[E|W])\\)");
        let location_parser = location_parser.map_err(|e| DataRecordParsingError::ParseFailure(format!("Failed to create location regex: {}", e)))?;

        let location_str = lines.next().ok_or(DataRecordParsingError::ParseFailure("Invalid data for location metadata".into()))?;
        let location = match location_parser.captures(location_str) {
            Some(captures) => {
                let name = captures.get(1).unwrap().as_str();
                let latitude_str = captures.get(2).unwrap().as_str();
                let longitude_str = captures.get(3).unwrap().as_str();

                let latitude = parse_latitude(latitude_str)?;
                let longitude = parse_longitude(longitude_str)?;

                Ok(Location::new(latitude, longitude, name.into()))
            },
            None => Err(DataRecordParsingError::ParseFailure("Failed to capture location data from regex".into())),
        }?;

        // Skip the second line
        lines.next();

        // The third has the model run date and time
        let model_run_parser = Regex::new("Cycle\\s*:\\s*([0-9]{0,4})([0-9]{0,2})([0-9]{0,2})\\s*([0-9]{0,2})");
        let model_run_parser = model_run_parser.map_err(|_| DataRecordParsingError::ParseFailure("Failed to create regex to parse model date run".into()))?;

        let model_run_date = match model_run_parser.captures(lines.next().unwrap_or("")) {
            Some(captures) => {
                let year = captures.get(1).unwrap().as_str().parse::<i32>().map_err(|_| DataRecordParsingError::ParseFailure("Failed to capture model date year".into()))?;
                let month = captures.get(2).unwrap().as_str().parse::<i32>().map_err(|_| DataRecordParsingError::ParseFailure("Failed to capture model date month".into()))?;
                let day = captures.get(3).unwrap().as_str().parse::<i32>().map_err(|_| DataRecordParsingError::ParseFailure("Failed to capture model date day".into()))?;
                let hour = captures.get(4).unwrap().as_str().parse::<i32>().map_err(|_| DataRecordParsingError::ParseFailure("Failed to capture model date hour".into()))?;
                let minute = 0;

                Ok(DateRecord{year, month, day, hour, minute})
            },
            None => Err(DataRecordParsingError::ParseFailure("Failed to capture model run date from regex".into())),
        }?;

        Ok(ForecastBulletinWaveRecordMetadata {
            location,
            model_run_date,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ForecastBulletinWaveRecord {
    pub date: DateRecord,
    pub significant_wave_height: DimensionalData<f64>,
    pub swell_components: Vec<Swell>,
}

impl ParseableDataRecord for ForecastBulletinWaveRecord {
    type Metadata = ForecastBulletinWaveRecordMetadata;

    fn from_data(data: &str) -> Result<(Option<Self::Metadata>, Vec<ForecastBulletinWaveRecord>), DataRecordParsingError> {
        Err(DataRecordParsingError::NotImplemented)
    }

    fn from_data_row(metadata: &Option<Self::Metadata>, row: &Vec<&str>) -> Result<Self, DataRecordParsingError>
    where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}

fn parse_latitude(raw: &str) -> Result<f64, DataRecordParsingError> {
    let latitude = raw[0..raw.len() - 1]
        .parse::<f64>()
        .map_err(|e| DataRecordParsingError::ParseFailure(format!("Failed to parse latitude: {:?}", e)))?;

    if raw.contains('S') {
        Ok(-latitude)
    } else {
        Ok(latitude)
    }
}

fn parse_longitude(raw: &str) -> Result<f64, DataRecordParsingError> {
    let longitude = raw[0..raw.len() - 1]
        .parse::<f64>()
        .map_err(|e| DataRecordParsingError::ParseFailure(format!("Failed to parse longitude: {:?}", e)))?;

    if raw.contains('W') {
        Ok(-longitude)
    } else {
        Ok(longitude)
    }
}

#[cfg(test)]
mod tests {
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
    fn parse_wave_bulletin_metadata() {
        let metadata = "Location : 44097      (40.98N  71.12W)
        Model    : spectral resolution for points
        Cycle    : 20220519 18 UTC
        ";

        let metadata = ForecastBulletinWaveRecordMetadata::from_str(metadata);

        println!("{:?}", metadata);
        assert!(metadata.is_ok());
    }

    #[test]
    fn test_wave_bulletin_row_parse() {

    }
}