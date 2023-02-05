use crate::{
    location::Location, model::ModelDataSource, station::Station,
    tools::{date::closest_gfs_model_datetime, dap::{DapConstraint, format_dods_url}},
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use geojson::{Feature, FeatureCollection, JsonObject, Geometry, JsonValue, Value};
use quick_xml::de::from_reader;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use std::{
    convert::{Into, TryInto},
    fmt,
    string::String,
};

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum BuoyType {
    None,
    Buoy,
    Fixed,
    OilRig,
    Dart,
    Tao,
    USV,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "station")]
pub struct BuoyStation {
    #[serde(rename = "id")]
    pub station_id: String,

    #[serde(rename = "name")]
    pub raw_name: String,

    pub owner: String,

    #[serde(rename = "pgm")]
    pub program: String,

    #[serde(rename = "type")]
    pub buoy_type: BuoyType,

    #[serde(rename = "met", deserialize_with = "bool_from_simple_str", default)]
    pub has_meteorological_data: bool,

    #[serde(
        default,
        rename = "currents",
        deserialize_with = "bool_from_simple_str"
    )]
    pub has_currents_data: bool,

    #[serde(
        rename = "waterquality",
        deserialize_with = "bool_from_simple_str",
        default
    )]
    pub has_water_quality_data: bool,

    #[serde(default, rename = "dart", deserialize_with = "bool_from_simple_str")]
    pub has_tsnuami_data: bool,

    #[serde(rename = "lat", deserialize_with = "f64_from_str")]
    pub latitude: f64,

    #[serde(rename = "lon", deserialize_with = "f64_from_str")]
    pub longitude: f64,

    #[serde(rename = "elev", deserialize_with = "f64_from_str", default)]
    pub elevation: f64,
}

impl BuoyStation {
    pub fn new(station_id: String, latitude: f64, longitude: f64) -> BuoyStation {
        BuoyStation {
            station_id: station_id,
            latitude,
            longitude,
            raw_name: "".into(),
            owner: String::from(""),
            program: String::from(""),
            buoy_type: BuoyType::Buoy,
            has_meteorological_data: true,
            has_currents_data: false,
            has_water_quality_data: false,
            has_tsnuami_data: false,
            elevation: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.has_meteorological_data
            || self.has_currents_data
            || self.has_water_quality_data
            || self.has_water_quality_data
    }

    pub fn latest_obs_data_url(&self) -> String {
        format!(
            "https://ndbc.noaa.gov/data/latest_obs/{}.txt",
            self.station_id
        )
    }

    pub fn meteorological_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.txt",
            self.station_id
        )
    }

    pub fn wave_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.spec",
            self.station_id
        )
    }

    pub fn spectral_wave_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.data_spec",
            self.station_id
        )
    }

    pub fn mean_wave_direction_spectral_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.swdir",
            self.station_id
        )
    }

    pub fn principal_wave_direction_spectral_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.swdir2",
            self.station_id
        )
    }

    pub fn r1_spectral_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.swr1",
            self.station_id
        )
    }

    pub fn r2_spectral_data_url(&self) -> String {
        format!(
            "https://www.ndbc.noaa.gov/data/realtime2/{}.swr2",
            self.station_id
        )
    }

    pub fn stdmet_dap_url_root(&self) -> String {
        format!(
            "https://dods.ndbc.noaa.gov/thredds/dodsC/data/stdmet/{station_id}/{station_id}h9999.nc",
            station_id = self.station_id,
        )
    }

    pub fn stdmet_das_url(&self) -> String {
        let root = self.stdmet_dap_url_root();
        format!("{root}.das")
    }

    pub fn stdmet_dds_url(&self) -> String {
        let root = self.stdmet_dap_url_root();
        format!("{root}.dds")
    }

    pub fn stdmet_dods_url(&self, constraints: &[DapConstraint]) -> String {
        format_dods_url(&self.stdmet_dap_url_root(), constraints)
    }

    pub fn swden_dap_url_root(&self) -> String {
        format!(
            "https://dods.ndbc.noaa.gov/thredds/dodsC/data/swden/{station_id}/{station_id}w9999.nc",
            station_id = self.station_id,
        )
    }

    pub fn swden_das_url(&self) -> String {
        let root = self.swden_dap_url_root();
        format!("{root}.das")
    }

    pub fn swden_dds_url(&self) -> String {
        let root = self.swden_dap_url_root();
        format!("{root}.dds")
    }

    pub fn swden_dods_url(&self, constraints: &[DapConstraint]) -> String {
        format_dods_url(&self.swden_dap_url_root(), constraints)
    }

    pub fn gfswave_bulletin_data_url(
        &self,
        source: &ModelDataSource,
        date: &DateTime<Utc>,
    ) -> String {
        let model_date = closest_gfs_model_datetime(date);
        format!(
            "{}/gfs.{}{:02}{:02}/{:02}/wave/station/bulls.t{:02}z/gfswave.{}.cbull",
            self.source_path(source),
            model_date.year(),
            model_date.month(),
            model_date.day(),
            model_date.hour(),
            model_date.hour(),
            self.station_id
        )
    }

    pub fn gfswave_spectral_data_url(
        &self,
        source: &ModelDataSource,
        date: &DateTime<Utc>,
    ) -> String {
        let model_date = closest_gfs_model_datetime(date);
        format!(
            "{}/gfs.{}{:02}{:02}/{:02}/wave/station/bulls.t{:02}z/gfswave.{}.spec",
            self.source_path(source),
            model_date.year(),
            model_date.month(),
            model_date.day(),
            model_date.hour(),
            model_date.hour(),
            self.station_id
        )
    }

    pub fn gfswave_lsl_url(&self, source: &ModelDataSource, date: &DateTime<Utc>) -> String {
        let model_date = closest_gfs_model_datetime(date);
        format!(
            "{}/gfs.{}{:02}{:02}/{:02}/wave/station/ls-l",
            self.source_path(source),
            model_date.year(),
            model_date.month(),
            model_date.day(),
            model_date.hour()
        )
    }

    fn source_path(&self, source: &ModelDataSource) -> &'static str {
        match source {
            ModelDataSource::NODDAWS => "https://noaa-gfs-bdp-pds.s3.amazonaws.com",
            ModelDataSource::NOMADS => "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod",
            ModelDataSource::NODDGCP => "https://storage.googleapis.com/global-forecast-system",
        }
    }
}

impl Station for BuoyStation {
    fn id(&self) -> &str {
        &self.station_id
    }

    fn location(&self) -> Location {
        Location::new(self.latitude, self.longitude, self.name())
    }

    fn name(&self) -> String {
        let mut name = self
            .raw_name
            .split("-")
            .map(|s| s.trim())
            .filter(|s| match s.parse::<i64>() {
                Ok(_) => false,
                _ => true,
            })
            .collect::<Vec<&str>>()
            .join("");

        name = name
            .split_whitespace()
            .filter(|s| if s.starts_with("(") { false } else { true })
            .collect::<Vec<&str>>()
            .join(" ");

        name
    }

    fn as_feature(&self) -> Feature {
        self.clone().into()
    }
}

impl Into<Feature> for BuoyStation {
    fn into(self) -> Feature {
        let lnglat: Vec<f64> = vec![self.longitude, self.latitude];
        let geometry = Geometry::new(Value::Point(lnglat));

        let mut properties = JsonObject::new();
        properties.insert("id".to_string(), JsonValue::from(self.id().to_string()));
        properties.insert("name".to_string(), JsonValue::from(self.name()));
        properties.insert("isActive".to_string(), JsonValue::from(self.is_active()));

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
pub struct BuoyStations {
    #[serde(rename = "$value")]
    pub stations: Vec<BuoyStation>,

    #[serde(rename = "count")]
    pub station_count: usize,
}

impl BuoyStations {
    pub fn active_stations_url() -> &'static str {
        "https://www.ndbc.noaa.gov/activestations.xml"
    }

    pub fn latest_obs_url() -> &'static str {
        "https://www.ndbc.noaa.gov/data/latest_obs/latest_obs.txt"
    }

    pub fn from_raw_data(raw_data: &str) -> Self {
        from_reader(raw_data.as_bytes()).unwrap()
    }

    pub fn from_stations(stations: Vec<BuoyStation>) -> Self {
        let stations_count = stations.len().try_into().unwrap();
        BuoyStations {
            stations: stations,
            station_count: stations_count,
        }
    }

    pub fn find_station(&self, station_id: &str) -> Option<BuoyStation> {
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

impl Default for BuoyStations {
    fn default() -> BuoyStations {
        BuoyStations {
            stations: vec![],
            station_count: 0,
        }
    }
}

impl From<Vec<BuoyStation>> for BuoyStations {
    fn from(stations: Vec<BuoyStation>) -> Self {
        BuoyStations::from_stations(stations)
    }
}

impl Into<FeatureCollection> for BuoyStations {
    fn into(self) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: self
                .stations
                .iter()
                .map(|s| s.clone().into())
                .collect::<Vec<Feature>>(),
            foreign_members: None,
        }
    }
}

struct NDBCBoolVisitor;

impl<'de> Visitor<'de> for NDBCBoolVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a boolean or a string containing 'y' or 'n'")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v.as_ref() {
            "y" => Ok(true),
            "n" => Ok(false),
            _ => Err(E::custom::<String>(
                "Invalid string value for deserializer".into(),
            )),
        }
    }
}

fn bool_from_simple_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_bool(NDBCBoolVisitor)
}

struct F64Visitor;

impl<'de> Visitor<'de> for F64Visitor {
    type Value = f64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a float string or as a floating point number")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<f64>().map_err(serde::de::Error::custom)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<f64>().map_err(serde::de::Error::custom)
    }
}

fn f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_f64(F64Visitor)
}
