use serde::{Serialize, Deserialize};

use crate::units::{Direction, Measurement, UnitConvertible, Units};
use std::fmt::{self, Display};
use std::option::Option;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct DimensionalData<T> {
    pub value: Option<T>,
    pub variable_name: String,
    pub measurement: Measurement,
    pub unit: Units,
}

impl <T> DimensionalData<T> where T: Display {
    pub fn unit_label(&self, abbrev: bool) -> &'static str {
        self.unit.label(&self.measurement, abbrev).into()
    }

    pub fn try_string(&self) -> Option<String> {
        match self.value {
            Some(_) => Some(self.to_string()),
            None => None,
        }
    }
}

impl <T> DimensionalData<T> where T: FromStr {
    pub fn from_raw_data(raw_data: &str, variable_name: String, measurement: Measurement, unit: Units) -> DimensionalData<T> {
        let parsed_value = raw_data.parse();
        DimensionalData {
            value: match parsed_value {
                Ok(val) => Some(val),
                Err(_) => None,
            },
            variable_name: variable_name,
            measurement: measurement,
            unit: unit,
        }
    }
}

impl UnitConvertible<DimensionalData<f64>> for DimensionalData<f64> {
    fn to_units(&mut self, new_units: &Units) {
        self.value = match self.value {
            Some(val) => Some(self.unit.convert(&self.measurement, new_units, val)),
            None => None,
        };
        self.unit = new_units.clone();
    }
}

impl UnitConvertible<DimensionalData<i64>> for DimensionalData<i64> {
    fn to_units(&mut self, new_units: &Units) {
        self.value = match self.value {
            Some(val) => Some(self.unit.convert(&self.measurement, new_units, val as f64) as i64),
            None => None,
        };
        self.unit = new_units.clone();
    }
}

impl UnitConvertible<DimensionalData<Direction>> for DimensionalData<Direction> {
    fn to_units(&mut self, new_units: &Units) {
        // Do nothing
        self.unit = new_units.clone();
    }
}

impl<T> fmt::Display for DimensionalData<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut label = self.unit.label(&self.measurement, true).to_owned();
        if label == "°" {
            label = "".to_owned();
        } else {
            label = format!(" {label}");
        }
        
        match self.value {
            Some(ref val) => write!(f, "{:.1}{}", val, label),
            None => write!(f, "N/A"),
        }
    }
}

impl <T> Serialize for DimensionalData<T> where T: Display + Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        
            #[derive(Serialize)]
            struct Extended<'a, T> {
                pub value: &'a Option<T>,
                pub variable_name: &'a String,
                pub measurement: &'a Measurement,
                pub unit: &'a Units,
                pub unit_label: &'a str, 
            }

            let ext = Extended {
                value: &self.value,
                variable_name: &self.variable_name,
                measurement: &self.measurement,
                unit: &self.unit,
                unit_label: self.unit_label(true), 
            };

            ext.serialize(serializer)
    }
}

pub enum DimensionalDataParseError {
    InvalidString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensional_data_serialize() {
        let dd = DimensionalData::<f64>::from_raw_data("4.0", "wave_height".to_string(), Measurement::Length, Units::English);
        let dd_s = serde_json::to_string(&dd);
        assert!(dd_s.is_ok());

        let dd_new = serde_json::from_str::<DimensionalData<f64>>(dd_s.unwrap().as_str());
        assert!(dd_new.is_ok());
    }
}