use std::fmt::Display;

use chrono::Utc;
use geojson::{Feature, Geometry, Value, JsonObject, JsonValue};
use serde::{Serialize, Deserialize};

use crate::{station::Station, location::Location, units::UnitSystem};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataInterval {
    Default,
    Hourly,
    HiLo,
}

impl Display for DataInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataInterval::Default => f.write_str(""),
            DataInterval::Hourly => f.write_str("h"),
            DataInterval::HiLo => f.write_str("hilo"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TideDatum {
    MHHW, 
    MHW, 
    MTL, 
    MSL, 
    MLW, 
    MLLW,
}

impl Display for TideDatum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TideDatum::MHHW => f.write_str("MHHW"),
            TideDatum::MHW => f.write_str("MHW"),
            TideDatum::MTL => f.write_str("MTL"),
            TideDatum::MSL => f.write_str("MSL"),
            TideDatum::MLW => f.write_str("MLW"),
            TideDatum::MLLW => f.write_str("MLLW"),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideStation {
    #[serde(rename = "id")]
    pub station_id: String,
    pub name: String,
    #[serde(rename = "lat")]
    pub latitude: f64,
    #[serde(rename = "lng")]
    pub longitude: f64,
    pub state: String,
    pub reference_id: String,
}

impl TideStation {
    pub fn new(station_id: &str, location: &Location, state: &str) -> Self {
        Self {
            station_id: station_id.to_string(),
            name: location.name.clone(),
            latitude: location.latitude,
            longitude: location.longitude,
            state: state.to_string(),
            reference_id: "".to_string(),
        }
    }

    pub fn tidal_data_url(&self, start_date: &chrono::DateTime<Utc>, end_date: &chrono::DateTime<Utc>, datum: &TideDatum, interval: &DataInterval, units: &UnitSystem) -> String {
        format!("https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?begin_date={0}%20{1}&end_date={2}%20{3}&station={4}&product=predictions&datum={5}&interval={6}&units={7}&time_zone=gmt&application=web_services&format=json", 
            start_date.format("%Y%m%d"), 
            start_date.format("%H:%M"), 
            end_date.format("%Y%m%d"), 
            end_date.format("%H:%M"), 
            self.station_id, 
            datum, 
            interval, 
            units
        )
    }
}

impl Station for TideStation {
    fn id(&self) -> &str {
        &self.station_id
    }

    fn location(&self) -> crate::location::Location {
        Location::new(self.latitude, self.longitude, self.name.clone())
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn as_feature(&self) -> geojson::Feature {
        self.clone().into()
    }
}

impl Into<Feature> for TideStation {
    fn into(self) -> Feature {
        let lnglat: Vec<f64> = vec![self.longitude, self.latitude];
        let geometry = Geometry::new(Value::Point(lnglat));

        let mut properties = JsonObject::new();
        properties.insert("id".to_string(), JsonValue::from(self.id().to_string()));
        properties.insert("name".to_string(), JsonValue::from(self.name()));
        properties.insert("state".to_string(), JsonValue::from(self.state));

        Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideStations {
    #[serde(rename="count")]
    pub station_count: usize, 
    pub stations: Vec<TideStation>,
}

impl TideStations {
    pub fn tide_prediction_stations_url() -> String {
        "https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations.json?type=tidepredictions&units=english".to_string()
    }

    pub fn from_raw_data(raw_data: &str) -> Self {
        serde_json::from_reader(raw_data.as_bytes()).unwrap()
    }

    pub fn from_stations(stations: Vec<TideStation>) -> Self {
        let stations_count = stations.len().try_into().unwrap();
        TideStations {
            stations: stations,
            station_count: stations_count,
        }
    }

    pub fn find_station_by_id(&self, station_id: &str) -> Option<TideStation> {
        match self
            .stations
            .iter()
            .position(|x| x.station_id == station_id)
        {
            Some(index) => Some(self.stations[index].clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Days;

    use super::*;

    #[test]
    pub fn parse_station() {
        let raw_station = r#"
        {
            "state": "RI",
            "tidepredoffsets": {
              "self": "https://api.tidesandcurrents.noaa.gov/mdapi/prod/webapi/stations/8452660/tidepredoffsets.json"
            },
            "type": "R",
            "timemeridian": 0,
            "reference_id": "8452660",
            "timezonecorr": -5,
            "id": "8452660",
            "name": "NEWPORT",
            "lat": 41.505,
            "lng": -71.3267,
            "affiliations": "",
            "portscode": "",
            "products": null,
            "disclaimers": null,
            "notices": null,
            "self": null,
            "expand": null,
            "tideType": ""
          }
        "#;

        let station = serde_json::from_str::<TideStation>(&raw_station);
        assert!(station.is_ok());

        let station = station.unwrap();

        let start_date = Utc::now();
        let end_date = start_date.checked_add_days(Days::new(7)).unwrap();
        let _ = station.tidal_data_url(&start_date, &end_date, &TideDatum::MLW, &DataInterval::Default, &UnitSystem::English);
    }
}