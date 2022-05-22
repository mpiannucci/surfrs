use std::str::FromStr;

use regex::Regex;

use crate::units::{Direction};
use crate::dimensional_data::DimensionalData;

use super::date_record::DateRecord;
use super::parseable_data_record::{ParseableDataRecord, DataRecordParsingError};


#[derive(Clone, Debug)]
pub struct ForecastSpectralWaveDataRecordMetadata {
    pub frequency: Vec<f64>,
    pub direction: Vec<Direction>,
    pub point_count: usize,
}

impl FromStr for ForecastSpectralWaveDataRecordMetadata {
    type Err = DataRecordParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let header_regex = Regex::new("'WAVEWATCH III SPECTRA'\\s*([0-9]{0,2})\\s*([0-9]{0,2})\\s*([0-9]{0,2})");
        let header_regex = header_regex.map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to create metadata header regex: {}", e))
        })?;
        
        Err(DataRecordParsingError::NotImplemented)
    }
}

#[derive(Clone, Debug)]
pub struct ForecastSpectralWaveDataRecord {
    pub date: DateRecord,
    pub depth: DimensionalData<f64>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub current_speed: DimensionalData<f64>, 
    pub current_direction: DimensionalData<Direction>,
    pub values: Vec<f64>,
}

impl ParseableDataRecord for ForecastSpectralWaveDataRecord {
    type Metadata = ForecastSpectralWaveDataRecordMetadata;

    fn from_data(data: &str, count: Option<usize>) -> Result<(Option<Self::Metadata>, Vec<ForecastSpectralWaveDataRecord>), DataRecordParsingError> {
        Err(DataRecordParsingError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}