use crate::data::units::*;

use super::meteorological_data_record::MeteorologicalDataRecord;
use super::wave_data_record::WaveDataRecord;
use super::spectral_wave_data_record::SpectralWaveDataRecord;
use super::parseable_data_record::ParseableDataRecord;

#[derive(Clone, Debug)]
pub enum BuoyDataRecord {
    Latest(MeteorologicalDataRecord, WaveDataRecord),
    Meteorological(MeteorologicalDataRecord), 
    Wave(WaveDataRecord),
    SprectralWave(SpectralWaveDataRecord),
}

impl UnitConvertible<BuoyDataRecord> for BuoyDataRecord {
    fn to_units(&mut self, new_units: &Units) {
        match self {
            BuoyDataRecord::Latest(met_data, wave_data) => {
                met_data.to_units(new_units);
                wave_data.to_units(new_units);
            }
            BuoyDataRecord::Meteorological(data) => data.to_units(new_units),
            BuoyDataRecord::Wave(data) => data.to_units(new_units),
            BuoyDataRecord::SprectralWave(data) => data.to_units(new_units),
        }
    }
}

impl BuoyDataRecord {
    pub fn parse_from_meteorological_data(raw_data: &str) -> Option<BuoyDataRecord> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(raw_data.as_bytes());

        let data: Vec<Option<MeteorologicalDataRecord>> = reader.records().map(|result| -> Option<MeteorologicalDataRecord> {
            if let Ok(record) = result {
                let filtered_record: Vec<&str> = record.iter().filter(|data| !data.is_empty()).collect();
                let mut met_data = MeteorologicalDataRecord::from_data_row(&filtered_record);
                met_data.to_units(&Units::Metric);
                return Some(met_data);
            }
            None
        }).filter(|result| {
            match result {
                Some(_) => true,
                None => false
            }
        }).collect();

        match data.first() {
            Some(d) => Some(BuoyDataRecord::Meteorological(d.clone().unwrap())),
            None => None
        }
    }

    pub fn parse_from_detailed_wave_data(raw_data: &str) -> Option<BuoyDataRecord> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(raw_data.as_bytes());

        let data: Vec<Option<WaveDataRecord>> = reader.records().map(|result| -> Option<WaveDataRecord> {
            if let Ok(record) = result {
                let filtered_record: Vec<&str> = record.iter().filter(|data| !data.is_empty()).collect();
                let mut wave_data = WaveDataRecord::from_data_row(&filtered_record);
                wave_data.to_units(&Units::Metric);
                return Some(wave_data);
            }
            None
        }).filter(|result| {
            match result {
                Some(_) => true,
                None => false
            }
        }).collect();

        match data.first() {
            Some(d) => Some(BuoyDataRecord::Wave(d.clone().unwrap())),
            None => None
        }
    }
}
