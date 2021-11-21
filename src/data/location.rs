use std::string::String;
use std::f64;
use serde::de::{self, Deserialize, Deserializer};
use crate::data::units::Units;
use serde_derive::*;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    #[serde(default)]
    pub name: String,

    #[serde(rename = "lat", deserialize_with = "f64_from_str")]
    pub latitude: f64,

    #[serde(rename = "lon", deserialize_with = "f64_from_str")]
    pub longitude: f64,

    #[serde(rename = "elev", deserialize_with = "f64_from_str", default)]
    pub altitude: f64,
}

impl Location {
    pub fn new(lat: f64, lon: f64, name: String) -> Location {
        Location {
            name: name,
            latitude: lat,
            longitude: lon,
            altitude: 0.0
        }
    }

    pub fn relative_latitude(&self) -> f64 {
        if self.latitude > 90.0 {
            self.latitude - 180.0
        } else {
            self.latitude
        }
    }

    pub fn relative_longitude(&self) -> f64 {
        if self.longitude > 180.0 {
            self.longitude - 360.0
        } else {
            self.longitude
        }
    }

    pub fn absolute_latitude(&self) -> f64 {
        if self.latitude < 0.0 {
            self.latitude + 180.0
        } else {
            self.latitude
        }
    }

    pub fn absolute_longitude(&self) -> f64 {
        if self.longitude < 0.0 {
            self.longitude + 360.0
        } else {
            self.longitude
        }
    }

    pub fn distance(&self, other: &Location, unit: &Units) -> f64 {
        let source_lat = self.absolute_latitude().to_radians();
        let source_lon = self.absolute_longitude().to_radians();
        let dest_lat = other.absolute_latitude().to_radians();
        let dest_lon = other.absolute_longitude().to_radians();

        // Compute using the haversine formula
        let d_lat = dest_lat - source_lat;
        let d_lon = dest_lon - source_lon;

        let a = (d_lat*0.5).powf(2.0).sin() + source_lat.cos() * dest_lat.cos() * (d_lon * 0.5).powf(2.0).sin();
        let c = 2.0 * a.sqrt().asin();
        let r = unit.earths_radius();

        c * r
    }
}

fn f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(de::Error::custom)
}