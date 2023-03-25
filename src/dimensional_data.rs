use serde::{Deserialize, Serialize};

use crate::units::{CardinalDirection, Direction, Unit, UnitConvertible, UnitSystem};
use std::fmt::{self, Display};
use std::option::Option;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct DimensionalData<T> {
    pub value: Option<T>,
    pub variable_name: String,
    pub unit: Unit,
}

impl DimensionalData<f64> {
    pub fn get_value(&self) -> f64 {
        self.value.unwrap_or(f64::NAN)
    }
}

impl DimensionalData<f32> {
    pub fn get_value(&self) -> f32 {
        self.value.unwrap_or(f32::NAN)
    }
}

impl DimensionalData<Direction> {
    pub fn get_value(&self) -> Direction {
        self.value
            .clone()
            .unwrap_or(Direction::from_cardinal_direction(
                CardinalDirection::Invalid,
            ))
    }
}

impl<T> DimensionalData<T>
where
    T: Display,
{
    pub fn unit_label(&self) -> &'static str {
        self.unit.abbreviation()
    }

    pub fn try_string(&self) -> Option<String> {
        match self.value {
            Some(_) => Some(self.to_string()),
            None => None,
        }
    }
}

impl<T> DimensionalData<T>
where
    T: FromStr,
{
    pub fn from_raw_data(raw_data: &str, variable_name: String, unit: Unit) -> DimensionalData<T> {
        let parsed_value = raw_data.parse();
        DimensionalData {
            value: match parsed_value {
                Ok(val) => Some(val),
                Err(_) => None,
            },
            variable_name: variable_name,
            unit: unit,
        }
    }
}

impl UnitConvertible<DimensionalData<f64>> for DimensionalData<f64> {
    fn to_units(&mut self, new_units: &UnitSystem) {
        let new_unit = self.unit.convert_system(&new_units);
        self.value = match self.value {
            Some(value) => Some(self.unit.convert(value, &new_unit)),
            None => None,
        };
        self.unit = new_unit;
    }
}

impl UnitConvertible<DimensionalData<i64>> for DimensionalData<i64> {
    fn to_units(&mut self, new_units: &UnitSystem) {
        let new_unit = self.unit.convert_system(&new_units);
        self.value = match self.value {
            Some(value) => Some(self.unit.convert(value as f64, &new_unit) as i64),
            None => None,
        };
        self.unit = new_unit;
    }
}

impl UnitConvertible<DimensionalData<Direction>> for DimensionalData<Direction> {
    fn to_units(&mut self, new_units: &UnitSystem) {
        // Do nothing
        self.unit = self.unit.convert_system(&new_units);
    }
}

impl From<DimensionalData<Direction>> for DimensionalData<f64> {
    fn from(data: DimensionalData<Direction>) -> Self {
        DimensionalData {
            value: data.value.map(|v| v.degrees as f64),
            variable_name: data.variable_name.clone(),
            unit: data.unit.clone(),
        }
    }
}

impl<T> fmt::Display for DimensionalData<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut label = self.unit.abbreviation().to_string();
        if label == "°" {
            label = "".into();
        } else {
            label = format!(" {label}");
        }

        match self.value {
            Some(ref val) => write!(f, "{:.1}{}", val, label),
            None => write!(f, "N/A"),
        }
    }
}

impl<T> Serialize for DimensionalData<T>
where
    T: Display + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Extended<'a, T> {
            pub value: &'a Option<T>,
            pub variable_name: &'a String,
            pub unit: &'a Unit,
            pub unit_label: &'a str,
        }

        let ext = Extended {
            value: &self.value,
            variable_name: &self.variable_name,
            unit: &self.unit,
            unit_label: self.unit_label(),
        };

        ext.serialize(serializer)
    }
}

pub struct DimensionalDataCollection<T>(Vec<DimensionalData<T>>);

impl<T> Into<Vec<Option<T>>> for DimensionalDataCollection<T>
where
    T: Clone,
{
    fn into(self) -> Vec<Option<T>> {
        self.0.into_iter().map(|d| d.value).collect()
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
        let dd =
            DimensionalData::<f64>::from_raw_data("4.0", "wave_height".to_string(), Unit::Meters);
        let dd_s = serde_json::to_string(&dd);
        assert!(dd_s.is_ok());

        let dd_new = serde_json::from_str::<DimensionalData<f64>>(dd_s.unwrap().as_str());
        assert!(dd_new.is_ok());
    }
}
