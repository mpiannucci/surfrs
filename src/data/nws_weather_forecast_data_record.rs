use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    dimensional_data::DimensionalData,
    units::{Direction, Unit},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NwsGridPointProperties {
    pub grid_id: String,
    pub grid_x: usize,
    pub grid_y: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NwsGridPointData {
    pub properties: NwsGridPointProperties,
    // Ignore everything else
}

impl NwsGridPointData {
    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WMODataField {
    unit_code: String,
    value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsWeatherForecastPeriodData {
    number: usize,
    name: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    is_daytime: bool,
    temperature: f64,
    temperature_unit: String,
    temperature_trend: Option<String>,
    probability_of_precipitation: WMODataField,
    dewpoint: WMODataField,
    relative_humidity: WMODataField,
    wind_speed: String,
    wind_direction: String,
    icon: String,
    short_forecast: String,
    detailed_forecast: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NwsWeatherForecastDataRecord {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_daytime: bool,
    pub temperature: DimensionalData<f64>,
    pub dewpoint: DimensionalData<f64>,
    pub humidity: DimensionalData<f64>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub icon: String,
    pub short_forecast: String,
    pub detailed_forecast: String,
}

impl From<&NwsWeatherForecastPeriodData> for NwsWeatherForecastDataRecord {
    fn from(data: &NwsWeatherForecastPeriodData) -> Self {
        let wind_speed_parts = data.wind_speed.split(" ").collect::<Vec<&str>>();

        Self {
            start_time: data.start_time,
            end_time: data.end_time,
            is_daytime: data.is_daytime,
            temperature: DimensionalData {
                value: Some(data.temperature),
                variable_name: "temperature".to_string(),
                unit: Unit::from(data.temperature_unit.as_str()),
            },
            dewpoint: DimensionalData {
                value: Some(data.dewpoint.value),
                variable_name: "dewpoint".to_string(),
                unit: Unit::from(data.dewpoint.unit_code.as_str()),
            },
            humidity: DimensionalData {
                value: Some(data.relative_humidity.value),
                variable_name: "humidity".to_string(),
                unit: Unit::from(data.relative_humidity.unit_code.as_str()),
            },
            wind_speed: DimensionalData::from_raw_data(
                wind_speed_parts[0],
                "wind speed".into(),
                Unit::from(wind_speed_parts[1]),
            ),
            wind_direction: DimensionalData::from_raw_data(
                &data.wind_direction,
                "wind direction".into(),
                Unit::Degrees,
            ),
            icon: data.icon.clone(),
            short_forecast: data.short_forecast.clone(),
            detailed_forecast: data.detailed_forecast.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsWeatherForecastDataRecordCollectionProperties {
    units: String,
    forecast_generator: String,
    generated_at: DateTime<Utc>,
    update_time: DateTime<Utc>,
    elevation: WMODataField,
    periods: Vec<NwsWeatherForecastPeriodData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NwsWeatherForecastDataRecordCollection {
    properties: NwsWeatherForecastDataRecordCollectionProperties,
}

impl NwsWeatherForecastDataRecordCollection {
    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }

    pub fn records(&self) -> Vec<NwsWeatherForecastDataRecord> {
        self.properties
            .periods
            .iter()
            .map(|record| NwsWeatherForecastDataRecord::from(record))
            .collect()
    }
}
