use crate::units::*;

use super::date_record::DateRecord;
use super::parseable_data_record::ParseableDataRecord;

pub struct ForecastSpectralWaveRecord {
    pub date: DateRecord,
    pub frequency: Vec<f64>,
    pub direction: Vec<f64>,
    pub values: Vec<f64>,
}