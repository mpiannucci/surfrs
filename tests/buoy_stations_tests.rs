extern crate surfrs;

use surfrs::buoy::BuoyStations;
use surfrs::data::latest_obs_data_record::{LatestObsDataRecordCollection, latest_obs_feature_collection};
use surfrs::station::Station;
use std::fs::{File, self};
use std::io::Read;
use std::path::Path;
use geojson::FeatureCollection;

fn read_stations_mock() -> String {
    let stations_xml_path = Path::new("mock/activestations.xml");
    let mut stations_xml_file = File::open(stations_xml_path).expect("file not found");
    let mut raw_station_data = String::new();
    stations_xml_file
        .read_to_string(&mut raw_station_data)
        .unwrap();
    raw_station_data
}

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_stations_xml() {
    let raw_station_data = read_stations_mock();
    let buoy_stations: BuoyStations = BuoyStations::from_raw_data(raw_station_data.as_ref());
    assert_eq!(
        buoy_stations.station_count,
        buoy_stations.stations.len() as i64 - 1
    );

    let bi_station_res = buoy_stations.find_station("44097");
    assert_eq!(bi_station_res.is_some(), true);

    if let Some(bi_station) = bi_station_res {
        assert_eq!(bi_station.name().as_str(), "Block Island, RI")
    }

    let serialized = serde_json::to_string(&buoy_stations);
    assert_eq!(serialized.is_ok(), true);
    let serialized = serialized.unwrap();
    let restored_stations = serde_json::from_str::<BuoyStations>(serialized.as_str());
    assert_eq!(restored_stations.is_ok(), true);

    let restored_stations = restored_stations.unwrap();
    assert_eq!(
        buoy_stations.station_count,
        restored_stations.station_count
    );
    assert_eq!(
        restored_stations.stations.len(),
        buoy_stations.stations.len()
    );
}

#[test]
fn read_stations_latest_observations() {
    let raw_station_data = read_stations_mock();
    let buoy_stations: BuoyStations = BuoyStations::from_raw_data(raw_station_data.as_ref());
    assert_eq!(
        buoy_stations.station_count,
        buoy_stations.stations.len() as i64 - 1
    );

    let raw_data = read_mock_data("latest_obs.txt");
    let mut data_collection = LatestObsDataRecordCollection::from_data(raw_data.as_str());
    let latest_obs_records = data_collection.records().collect();

    let feature_collection = latest_obs_feature_collection(&buoy_stations, &latest_obs_records);
    let serialized_feature_collection = serde_json::to_string(&feature_collection);
    assert!(serialized_feature_collection.is_ok());

    let serialized_feature_collection = serialized_feature_collection.unwrap();
    assert!(!serialized_feature_collection.contains("null"));

    let deserialized_feature_collection = serde_json::from_str::<FeatureCollection>(&serialized_feature_collection);
    assert!(deserialized_feature_collection.is_ok());
}