use crate::units::*;

use super::date_record::DateRecord;
use super::parseable_data_record::ParseableDataRecord;

#[Derive(Clone, Debug)]
pub struct ForecastSpectralWaveRecord {
    pub date: DateRecord,
    pub frequency: Vec<f64>,
    pub direction: Vec<f64>,
    pub values: Vec<f64>,
}

impl ForecastSpectralWaveRecord {
    pub fn frequency_count(&self) -> usize {
        self.frequency.len()
    }

    pub fn direction_count(&self) -> usize {
        self.direction.len()
    }

    pub fn data_count(&self) -> usize {
        self.frequency_count() * self.direction_count()
    }
}

impl ParseableDataRecord for ForecastSpectralWaveRecord {
    fn from_data(data: &str) -> Result<Vec<ForecastSpectralWaveRecord>, DataRecordParsingError> {
        Err(DataRecordParsingError::NotImplemented)
    }
}