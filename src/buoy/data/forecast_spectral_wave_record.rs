use crate::units::{DimensionalData, Direction};

use super::date_record::DateRecord;
use super::parseable_data_record::ParseableDataRecord;

#[Derive(Clone, Debug)]
pub struct ForecastSpectralWaveRecord {
    pub date: DateRecord,
    pub depth: DimensionalData<f64>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub current_speed: DimensionalData<f64>, 
    pub current_direction: DimensionalData<Direction>,
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