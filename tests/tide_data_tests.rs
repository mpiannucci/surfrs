use std::fs;

use surfrs::data::tidal_data_record::TidalDataRecordCollection;

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_default_prediction_data() {
    let raw_data = read_mock_data("default_tide_data.json");
    let data_collection = TidalDataRecordCollection::from_json(&raw_data);
    assert!(data_collection.is_ok());

    let data_collection = data_collection.unwrap();
    assert!(data_collection.records.len() > 0);
}

#[test]
fn read_hilo_prediction_data() {
    let raw_data = read_mock_data("hilo_tide_data.json");
    let data_collection = TidalDataRecordCollection::from_json(&raw_data);
    assert!(data_collection.is_ok());

    let data_collection = data_collection.unwrap();
    assert!(data_collection.records.len() > 0);
}
