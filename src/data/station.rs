use crate::data::location::Location;
use std::string::String;
use serde::de::{Deserialize, Deserializer};
use serde_xml_rs::de::from_reader;
use serde_derive::*;

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuoyType {
    None,
    Buoy, 
    Fixed,
    OilRig,
    Dart,
    Tao,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "station")]
pub struct BuoyStation {
    #[serde(rename = "id")]
    pub station_id: String,

    pub owner: String, 

    #[serde(rename = "pgm")]
    pub program: String,

    #[serde(rename = "type")]
    pub buoy_type: BuoyType,

    #[serde(rename = "met", deserialize_with = "bool_from_simple_str", default)]
    pub has_meteorological_data: bool,

    #[serde(default, rename="currents", deserialize_with = "bool_from_simple_str")]
    pub has_currents_data: bool,

    #[serde(rename = "waterquality", deserialize_with = "bool_from_simple_str", default)]
    pub has_water_quality_data: bool,

    #[serde(default, rename="dart", deserialize_with = "bool_from_simple_str")]
    pub has_tsnuami_data: bool,

    #[serde(flatten)]
    pub location: Location,
}

impl BuoyStation {
    pub fn new(station_id: String, location: Location) -> BuoyStation {
        BuoyStation {
            station_id: station_id,
            location: location,
            owner: String::from(""),
            program: String::from(""),
            buoy_type: BuoyType::Buoy,
            has_meteorological_data: true,
            has_currents_data: false,
            has_water_quality_data: false,
            has_tsnuami_data: false,
        }
    }

    pub fn name(&self) -> String {
        let mut name = self.location.name.split("-").map(|s| {
            s.trim()
        }).filter(|s| {
            match s.parse::<i64>() {
                Ok(_) => false, 
                _ => true
            }
        }).collect::<Vec<&str>>().join("");

        name = name.split_whitespace().filter(|s| {
            if s.starts_with("(") {
                false
            } else {
                true
            }
        }).collect::<Vec<&str>>().join(" ");

        name
    }

    pub fn is_active(&self) -> bool {
        self.has_meteorological_data || 
        self.has_currents_data || 
        self.has_water_quality_data || 
        self.has_water_quality_data
    }

    pub fn latest_obs_data_url(&self) -> String {
        format!("https://ndbc.noaa.gov/data/latest_obs/{}.txt", self.station_id)
    }

    pub fn meteorological_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.txt", self.station_id)
    }

    pub fn wave_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.spec", self.station_id)
    }

    pub fn spectral_wave_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.data_spec", self.station_id)
    }

    pub fn primary_spectral_wave_direction_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.swdir", self.station_id)
    }

    pub fn secondary_spectral_wave_direction_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.swdir2", self.station_id)
    }

    pub fn primary_spectral_wave_energy_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.swr1", self.station_id)
    }

    pub fn secondary_spectral_wave_energy_data_url(&self) -> String {
        format!("https://www.ndbc.noaa.gov/data/realtime2/{}.swr2", self.station_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuoyStations {
    #[serde(rename = "$value")]
    pub stations: Vec<BuoyStation>,

    #[serde(rename = "count")]
    pub station_count: i64,
}

impl BuoyStations {
    pub fn active_stations_url() -> String {
        String::from("https://www.ndbc.noaa.gov/activestations.xml")
    }

    pub fn from_raw_data(raw_data: &str) -> BuoyStations {
        from_reader(raw_data.as_bytes()).unwrap()
    }

    pub fn find_station(&self, station_id: &str) -> Option<BuoyStation> {
        match self.stations.iter().position(|x| x.station_id == station_id) {
            Some(index) => Some(self.stations[index].clone()),
            _ => None,
        }
    }
}

impl Default for BuoyStations {
    fn default() -> BuoyStations {
        BuoyStations {
            stations: vec![],
            station_count: 0,
        }
    }
}

fn bool_from_simple_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    match s.as_ref() {
        "y" => Ok(true),
        _ => Ok(false)
    }
}
