extern crate surfrs;

use std::fs;
use surfrs::data::forecast_cbulletin_wave_data_record::{ForecastCBulletinWaveRecordCollection, ForecastCBulletinWaveRecord};
use surfrs::data::forecast_spectral_wave_data_record::{ForecastSpectralWaveDataRecordCollection, ForecastSpectralWaveDataRecord};
use surfrs::data::meteorological_data_record::MeteorologicalDataRecordCollection;
use surfrs::data::spectral_wave_data_record::{SpectralWaveDataRecordCollection, DirectionalSpectralWaveDataRecord};
use surfrs::data::wave_data_record::{WaveDataRecordCollection};
use surfrs::swell::SwellProvider;

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_meteorological_data() {
    let raw_data = read_mock_data("44017.met.txt");
    let mut data_collection = MeteorologicalDataRecordCollection::from_data(raw_data.as_str());
    let data_records = data_collection.records();
    
    assert_eq!(data_records.count(), 1099);
}

#[test]
fn read_wave_data() {
    let raw_data = read_mock_data("44097.spec");
    let mut data_collection = WaveDataRecordCollection::from_data(raw_data.as_str());
    let first_record = data_collection.records().next();

    assert!(first_record.is_some())
}

#[test]
fn read_wave_energy_data() {
    let raw_data = read_mock_data("44008.data_spec");
    let mut data_collection = SpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let records = data_collection.records();
    
    assert!(records.count() == 1084);
}

#[test]
fn read_wave_direction_data() {
    let raw_data = read_mock_data("44097.swdir");
    let mut data_collection = SpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let records = data_collection.records();

    assert!(records.count() == 1099);
}

#[test]
fn read_wave_spectra_data() {
    let raw_energy_data = read_mock_data("44097.data_spec");
    let raw_mean_wave_direction_data = read_mock_data("44097.swdir");
    let raw_primary_wave_direction_data = read_mock_data("44097.swdir2");
    let raw_first_polar_coefficient_data = read_mock_data("44097.swr1");
    let raw_second_polar_coefficient_data = read_mock_data("44097.swr2");

    let mut energy_data_collection = SpectralWaveDataRecordCollection::from_data(raw_energy_data.as_str());
    let mut mean_wave_direction_data_collection = SpectralWaveDataRecordCollection::from_data(&raw_mean_wave_direction_data.as_str());
    let mut primary_wave_direction_data_collection = SpectralWaveDataRecordCollection::from_data(&raw_primary_wave_direction_data.as_str());
    let mut first_polar_coefficient_collection = SpectralWaveDataRecordCollection::from_data(&raw_first_polar_coefficient_data.as_str());
    let mut second_polar_coefficient_collection = SpectralWaveDataRecordCollection::from_data(&raw_second_polar_coefficient_data.as_str());
    
    let mut records = itertools::izip!(
        energy_data_collection.records(), 
        mean_wave_direction_data_collection.records(), 
        primary_wave_direction_data_collection.records(), 
        first_polar_coefficient_collection.records(), 
        second_polar_coefficient_collection.records(),
    )
        .map(|(e, mwd, pwd, r1, r2)| DirectionalSpectralWaveDataRecord::from_data(e, mwd, pwd, r1, r2));

    let swell_components = records
        .next()
        .unwrap()
        .swell_data()
        .unwrap()
        .components
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let control = "0.7 m @ 4.5 s 168° sse, 0.6 m @ 12.5 s 120° ese, 0.6 m @ 10.5 s 112° ese, 0.5 m @ 3.8 s 160° sse";
    let out = swell_components.join(", ");

    assert_eq!(out, control);
}

#[test]
fn read_cbulletin_forecast_station_data() {
    let raw_data = read_mock_data("gfswave.44097.cbull");
    let mut data_collection = ForecastCBulletinWaveRecordCollection::from_data(raw_data.as_str());
    let bulletin_records_iter = data_collection.records();
    assert!(bulletin_records_iter.is_ok());

    let bulletin_records: Vec<ForecastCBulletinWaveRecord> = bulletin_records_iter.unwrap().1.collect();
    assert!(bulletin_records[0].swell_data().is_ok());
    for (_, record) in bulletin_records.iter().enumerate() {
        assert!(record.swell_data().is_ok());
    }
}

#[test]
fn read_spectral_forecast_station_data() {
    let raw_data = read_mock_data("gfswave.44097.spec");
    let mut data_collection = ForecastSpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let spectral_records_iter = data_collection.records();
    assert!(spectral_records_iter.is_ok());

    let spectral_records: Vec<ForecastSpectralWaveDataRecord> = spectral_records_iter.unwrap().1.collect();
    assert_eq!(spectral_records.len(), 385);

    for (i, s) in spectral_records.iter().enumerate() {
        let spectra_swell_data = s.swell_data().is_ok();
    }
}