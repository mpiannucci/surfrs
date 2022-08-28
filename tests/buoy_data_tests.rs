extern crate surfrs;

use std::fs;
use surfrs::data::forecast_bulletin_wave_data_record::{ForecastBulletinWaveRecordCollection, ForecastBulletinWaveRecord};
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
    let raw_direction_data = read_mock_data("44097.swdir");

    let mut energy_data_collection = SpectralWaveDataRecordCollection::from_data(raw_energy_data.as_str());
    let mut direction_data_collection = SpectralWaveDataRecordCollection::from_data(raw_direction_data.as_str());

    let mut records = energy_data_collection.records()
        .zip(direction_data_collection.records())
        .map(|(e, d)| DirectionalSpectralWaveDataRecord::from_data(e, d));

    let swell_components = records
        .next()
        .unwrap()
        .swell_data()
        .unwrap()
        .components
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let control = "0.5 m @ 6.2 s 156° sse, 0.4 m @ 4.5 s 220° sw, 0.2 m @ 13.3 s 136° se";
    let out = swell_components.join(", ");

    assert_eq!(out, control);
}

#[test]
fn read_forecast_station_data() {
    let raw_data = read_mock_data("gfswave.44097.spec");
    let mut data_collection = ForecastSpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let spectral_records_iter = data_collection.records();
    assert!(spectral_records_iter.is_ok());

    let spectral_records: Vec<ForecastSpectralWaveDataRecord> = spectral_records_iter.unwrap().1.collect();
    assert_eq!(spectral_records.len(), 385);

    let spectral_wave_summary = spectral_records[0].swell_data();

    assert!(spectral_wave_summary.is_ok());
    for (_, record) in spectral_records.iter().enumerate() {
        assert!(record.swell_data().is_ok());
    }

    let raw_data = read_mock_data("gfswave.44097.cbull");
    let mut data_collection = ForecastBulletinWaveRecordCollection::from_data(raw_data.as_str());
    let bulletin_records_iter = data_collection.records();
    assert!(bulletin_records_iter.is_ok());

    let bulletin_records: Vec<ForecastBulletinWaveRecord> = bulletin_records_iter.unwrap().1.collect();
    assert!(bulletin_records[0].swell_data().is_ok());
    for (_, record) in bulletin_records.iter().enumerate() {
        assert!(record.swell_data().is_ok());
    }

    let zipped = bulletin_records.iter().zip(spectral_records.iter());
    for (bulletin, spectral) in zipped {
        println!("b: {}", bulletin.swell_data().unwrap().summary);
        println!("s: {}", spectral.swell_data().unwrap().summary);
        // println!("spart: {}", spectral.extract_partitions());
        // println!("-----------------------------------------------")
    }
}