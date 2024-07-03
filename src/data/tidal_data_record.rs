
use chrono::prelude::*;
use serde::{Serialize, Deserialize, de, Deserializer};
use serde_json::Value;

// https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?begin_date=20230102%2021:10&end_date=20230110%2021:10&station=8454658&product=predictions&datum=MTL&interval=&units=english&time_zone=gmt&application=web_services&format=json

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TidalEvent {
    #[serde(rename = "H")]
    High,
    #[serde(rename = "L")]
    Low,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TidalDataRecord {
    #[serde(rename = "t", deserialize_with = "utc_date_time_from_str")]
    pub date: chrono::DateTime<Utc>,
    #[serde(rename = "v", deserialize_with = "tidal_value_f64")]
    pub value: f64,
    #[serde(rename = "type")]
    pub event: Option<TidalEvent>,
}

fn utc_date_time_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let naive = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M").map_err(de::Error::custom)?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn tidal_value_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num.as_f64().ok_or(de::Error::custom("Invalid f64"))? as f64,
        _ => return Err(de::Error::custom("Invalid type for f64"))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalDataRecordCollection {
    #[serde(rename = "predictions")]
    pub records: Vec<TidalDataRecord>,
}

impl TidalDataRecordCollection {
    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;
    use chrono::Timelike;

    use crate::data::tidal_data_record::TidalEvent;

    use super::TidalDataRecord;

    #[test]
    fn deserialize() {
        let default_raw_data = r#"{"t":"2023-01-02 21:06", "v": "0.873"}"#;
        let default_data = serde_json::from_str::<TidalDataRecord>(default_raw_data).unwrap();
        assert_eq!(default_data.date.year(), 2023);
        assert_eq!(default_data.date.month(), 01);
        assert_eq!(default_data.date.day(), 02);
        assert_eq!(default_data.date.hour(), 21);
        assert_eq!(default_data.date.minute(), 06);
        assert!(default_data.value - 0.873 < 0.0000001);
        assert!(default_data.event.is_none());

        let hilo_raw_data = r#"{"t":"2023-01-02 21:36", "v":"0.932", "type":"H"}"#;
        let hilo_data = serde_json::from_str::<TidalDataRecord>(hilo_raw_data).unwrap();
        assert_eq!(hilo_data.date.year(), 2023);
        assert_eq!(hilo_data.date.month(), 01);
        assert_eq!(hilo_data.date.day(), 02);
        assert_eq!(hilo_data.date.hour(), 21);
        assert_eq!(hilo_data.date.minute(), 36);
        assert!(hilo_data.value - 0.932 < 0.0000001);
        assert!(hilo_data.event.is_some());
        assert_eq!(hilo_data.event, Some(TidalEvent::High));
    }
}
