use std::{path::Path, fs::File, io::Read};

use surfrs::tide::tide_station::TideStations;

extern crate surfrs;

fn read_stations_mock() -> String {
    let stations_json_path = Path::new("mock/tidalstations.json");
    let mut stations_json_file = File::open(stations_json_path).expect("file not found");
    let mut raw_station_data = String::new();
    stations_json_file
        .read_to_string(&mut raw_station_data)
        .unwrap();
    raw_station_data
}

#[test]
fn read_stations_json() {
    let raw_station_data = read_stations_mock();
    let tide_stations = TideStations::from_raw_data(&raw_station_data);
    assert_eq!(
        tide_stations.station_count,
        tide_stations.stations.len()
    );

    let station_id = "8454658";
    let narragansett_pier_station = tide_stations.find_station_by_id(&station_id);
    assert!(narragansett_pier_station.is_some());

    let narragansett_pier_station = narragansett_pier_station.unwrap();
    assert_eq!(narragansett_pier_station.station_id, station_id);
    assert_eq!(narragansett_pier_station.state, "RI");
    assert_eq!(narragansett_pier_station.name, "Narragansett Pier");
}