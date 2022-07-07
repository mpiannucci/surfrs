use crate::units::Units;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};
use std::{f64, fmt};
use std::string::String;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    #[serde(default)]
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
}

impl Location {
    pub fn new(lat: f64, lon: f64, name: String) -> Location {
        Location {
            name: name,
            latitude: lat,
            longitude: lon,
            elevation: 0.0,
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

        let a = (d_lat * 0.5).powf(2.0).sin()
            + source_lat.cos() * dest_lat.cos() * (d_lon * 0.5).powf(2.0).sin();
        let c = 2.0 * a.sqrt().asin();
        let r = unit.earths_radius();

        c * r
    }
}

