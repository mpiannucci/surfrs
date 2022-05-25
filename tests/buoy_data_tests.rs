extern crate surfrs;

use std::fs;
use surfrs::data::forecast_bulletin_wave_data_record::ForecastBulletinWaveRecord;
use surfrs::data::forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecord;
use surfrs::data::meteorological_data_record::MeteorologicalDataRecord;
use surfrs::data::parseable_data_record::ParseableDataRecord;
use surfrs::data::spectral_wave_data_record::SpectralWaveDataRecord;
use surfrs::data::wave_data_record::WaveDataRecord;

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_meteorological_data() {
    let raw_data = read_mock_data("44017.met.txt");
    let read_data = MeteorologicalDataRecord::from_data(raw_data.as_str(), None);
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 1099);
}

#[test]
fn read_wave_data() {
    let raw_data = read_mock_data("44097.spec");
    let read_data = WaveDataRecord::from_data(raw_data.as_str(), Some(1));
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 1);
}

#[test]
fn read_wave_energy_data() {
    let raw_data = read_mock_data("44008.data_spec");
    let read_data = SpectralWaveDataRecord::from_data(raw_data.as_str(), None);
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 1084);
}

#[test]
fn read_wave_direction_data() {
    let raw_data = read_mock_data("44097.swdir");
    let read_data = SpectralWaveDataRecord::from_data(raw_data.as_str(), None);
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 1098);
}

#[test]
fn read_forecast_bulletin_data() {
    let raw_data = read_mock_data("gfswave.44097.cbull");
    let read_data = ForecastBulletinWaveRecord::from_data(raw_data.as_str(), Some(5));
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 5);
}

#[test]
fn read_forecast_spectral_data() {
    let raw_data = read_mock_data("gfswave.44097.spec");
    let read_data = ForecastSpectralWaveDataRecord::from_data(raw_data.as_str(), None);
    
    assert_eq!(read_data.is_ok(), true);
    assert!(read_data.unwrap().1.len() == 385);
}