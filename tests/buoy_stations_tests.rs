extern crate surfrs;

use surfrs::buoy::BuoyStations;
use surfrs::station::Station;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn read_stations_mock() -> String {
    let stations_xml_path = Path::new("mock/activestations.xml");
    let mut stations_xml_file = File::open(stations_xml_path).expect("file not found");
    let mut raw_station_data = String::new();
    stations_xml_file
        .read_to_string(&mut raw_station_data)
        .unwrap();
    raw_station_data
}

#[test]
fn read_stations_xml() {
    let raw_station_data = read_stations_mock();
    let buoy_stations: BuoyStations = BuoyStations::from_raw_data(raw_station_data.as_ref());
    //println!("{:?}", buoy_stations);
    assert_eq!(
        buoy_stations.station_count,
        buoy_stations.stations.len() as i64 - 1
    );

    let bi_station_res = buoy_stations.find_station("44097");
    assert_eq!(bi_station_res.is_some(), true);

    if let Some(bi_station) = bi_station_res {
        assert_eq!(bi_station.name().as_str(), "Block Island, RI")
    }
}
