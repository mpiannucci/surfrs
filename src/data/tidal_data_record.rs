

// https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?begin_date=20230102%2021:10&end_date=20230110%2021:10&station=8454658&product=predictions&datum=MTL&interval=&units=english&time_zone=gmt&application=web_services&format=json

use chrono::Utc;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TidalEvent {
    #[serde(rename = "H")]
    High, 
    #[serde(rename = "L")]
    Low, 
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TidalDataRecord {
    #[serde(rename = "t")]
    date: chrono::DateTime<Utc>,
    #[serde(rename = "v")]
    value: f64,
    #[serde(rename = "type")]
    event: Option<TidalEvent>,
}