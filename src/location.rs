use crate::units::UnitSystem;
use serde::{Deserialize, Serialize};
use std::f64;
use std::string::String;

pub fn normalize_latitude(latitude: f64) -> f64 {
    if latitude > 90.0 {
        latitude - 180.0
    } else {
        latitude
    }
}

pub fn normalize_longitude(longitude: f64) -> f64 {
    if longitude > 180.0 {
        longitude - 360.0
    } else {
        longitude
    }
}

pub fn absolute_longitude(longitude: f64) -> f64 {
    if longitude < 0.0 {
        360.0 + longitude
    } else {
        longitude
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
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

    /// Great circle distance using the haversine formula, in km (metric) or
    /// miles (english).
    pub fn distance(&self, other: &Location, unit: &UnitSystem) -> f64 {
        let source_lat = self.latitude.to_radians();
        let dest_lat = other.latitude.to_radians();

        // Longitudes in -180..180 or 0..360 give the same result: sin² has a
        // period of 360 degrees in the longitude difference.
        let d_lat = dest_lat - source_lat;
        let d_lon = (other.longitude - self.longitude).to_radians();

        let a = (d_lat * 0.5).sin().powi(2)
            + source_lat.cos() * dest_lat.cos() * (d_lon * 0.5).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();
        let r = unit.earths_radius();

        c * r
    }

    pub fn within_bbox(&self, bbox: &(f64, f64, f64, f64)) -> bool {
        let within_lng = absolute_longitude(bbox.0) <= absolute_longitude(self.longitude)
            && absolute_longitude(self.longitude) <= absolute_longitude(bbox.2);
        let within_lat = normalize_latitude(bbox.1) <= self.latitude
            && self.latitude <= normalize_latitude(bbox.3);
        within_lng && within_lat
    }
}

#[cfg(test)]
mod tests {
    use crate::location::{absolute_longitude, normalize_latitude};

    use super::{normalize_longitude, Location};

    #[test]
    fn test_normalize_coords() {
        let normal_start_lng = normalize_longitude(260.0);
        assert!((normal_start_lng - -100.0).abs() < 0.00001);

        let normal_start_lng = normalize_longitude(90.0);
        assert!((normal_start_lng - 90.0).abs() < 0.00001);

        let normal_start_lat = normalize_latitude(100.0);
        assert!((normal_start_lat - -80.0).abs() < 0.00001);

        let normal_start_lat = normalize_latitude(85.0);
        assert!((normal_start_lat - 85.0).abs() < 0.00001);

        let normal_start_lng = normalize_longitude(0.0);
        assert!((normal_start_lng - 0.0).abs() < 0.00001);

        let normal_start_lng = normalize_longitude(359.75);
        assert!((normal_start_lng - -0.25).abs() < 0.00001);

        let normal_start_lat = normalize_latitude(-90.0);
        assert!((normal_start_lat - -90.0).abs() < 0.00001);

        let normal_start_lat = normalize_latitude(90.0);
        assert!((normal_start_lat - 90.0).abs() < 0.00001);
    }

    #[test]
    fn test_absolute_longitude() {
        let normal_start_lng = absolute_longitude(260.0);
        assert!((normal_start_lng - 260.0).abs() < 0.00001);

        let normal_start_lng = absolute_longitude(90.0);
        assert!((normal_start_lng - 90.0).abs() < 0.00001);

        let normal_start_lng = absolute_longitude(0.0);
        assert!((normal_start_lng - 0.0).abs() < 0.00001);

        let normal_start_lng = absolute_longitude(-71.4);
        assert!((normal_start_lng - 288.6).abs() < 0.00001);
    }

    #[test]
    fn test_distance() {
        use crate::units::UnitSystem;

        // London to Paris is about 343.5 km.
        let london = Location::new(51.5074, -0.1278, "London".into());
        let paris = Location::new(48.8566, 2.3522, "Paris".into());
        let d = london.distance(&paris, &UnitSystem::Metric);
        assert!((d - 343.5).abs() < 1.0, "{d}");
        assert!((paris.distance(&london, &UnitSystem::Metric) - d).abs() < 1e-9);

        // Sydney to Auckland is about 2156 km, across the southern hemisphere.
        let sydney = Location::new(-33.8688, 151.2093, "Sydney".into());
        let auckland = Location::new(-36.8485, 174.7633, "Auckland".into());
        let d = sydney.distance(&auckland, &UnitSystem::Metric);
        assert!((d - 2156.0).abs() < 5.0, "{d}");

        // Longitude convention does not matter.
        let buoy = Location::new(40.97, -71.13, "44097".into());
        let buoy_360 = Location::new(40.97, 288.87, "44097".into());
        let block_island = Location::new(41.17, -71.58, "Block Island".into());
        assert!(
            (buoy.distance(&block_island, &UnitSystem::Metric)
                - buoy_360.distance(&block_island, &UnitSystem::Metric))
            .abs()
                < 1e-9
        );
        assert_eq!(buoy.distance(&buoy, &UnitSystem::Metric), 0.0);
    }

    #[test]
    fn test_within_bbox() {
        let location = Location::new(41.35, -71.4, "Block Island Sound".into());
        let bbox = (260.0, -0.00010999999999228294, 310.0001, 55.0);
        assert!(location.within_bbox(&bbox));

        let bbox = (0.0, -90.0, 359.75, 90.0);
        assert!(location.within_bbox(&bbox));
    }
}
