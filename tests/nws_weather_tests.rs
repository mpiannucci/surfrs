use std::fs;

use surfrs::{data::nws_weather_forecast_data::{NwsWeatherForecastDataRecordCollection, NwsGridPointData}, units::{Unit, CardinalDirection, Direction}};

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_gridpoints_data() {
    let raw_data = read_mock_data("nws_gridpoints.json");
    let gridpoints = NwsGridPointData::from_json(&raw_data).unwrap();
    assert_eq!(gridpoints.properties.grid_id, "BOX");
    assert_eq!(gridpoints.properties.grid_x, 66);
    assert_eq!(gridpoints.properties.grid_y, 45);
}

#[test]
fn read_hourly_forecast_data() {
    let raw_data = read_mock_data("nws_hourly.json");
    let data_collection = NwsWeatherForecastDataRecordCollection::from_json(&raw_data);
    assert!(data_collection.is_ok());

    let data_collection = data_collection.unwrap();
    let records = data_collection.records();
    assert!(records.len() > 0);

    let temperature = &records[0].temperature;
    assert!((temperature.value.unwrap_or(0.0) - 38.0).abs() < 0.0001);
    assert_eq!(temperature.unit, Unit::Fahrenheit);
    
    let dewpoint = &records[0].dewpoint;
    assert!((dewpoint.value.unwrap_or(0.0) - 0.55555555555555558).abs() < 0.0001);
    assert_eq!(dewpoint.unit, Unit::Celsius);

    let humidity = &records[0].humidity;
    assert!((humidity.value.unwrap_or(0.0) - 82.0).abs() < 0.0001);
    assert_eq!(humidity.unit, Unit::Percent);

    let wind_speed = &records[0].wind_speed;
    assert!((wind_speed.value.unwrap_or(0.0) - 22.0).abs() < 0.0001);
    assert_eq!(wind_speed.unit, Unit::MilesPerHour);

    let wind_direction = &records[0].wind_direction;
    let wind_direction_value = wind_direction.value.as_ref().map(|d| d.cardinal_direction()).unwrap_or(&CardinalDirection::Invalid);
    assert!(wind_direction_value == &CardinalDirection::North);
}
