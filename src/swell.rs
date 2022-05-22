use crate::dimensional_data::DimensionalData;
use crate::units::{Units, Measurement, Direction, UnitConvertible};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Swell {
    pub wave_height: DimensionalData<f64>,
    pub period: DimensionalData<f64>,
    pub direction: DimensionalData<Direction>,
}

impl Swell {
    pub fn new(units: &Units, wave_height: f64, period: f64, direction: Direction) -> Swell {
        Swell {
            wave_height: DimensionalData {
                value: Some(wave_height),
                variable_name: "wave height",
                measurement: Measurement::Length,
                unit: units.clone(),
            },
            period: DimensionalData {
                value: Some(period),
                variable_name: "period",
                measurement: Measurement::Time,
                unit: units.clone(),
            },
            direction: DimensionalData {
                value: Some(direction),
                variable_name: "direction",
                measurement: Measurement::Direction,
                unit: units.clone(),
            }
        }
    }
}

impl UnitConvertible<Swell> for Swell {
    fn to_units(&mut self, new_units: &Units) {
        self.wave_height.to_units(new_units);
        self.wave_height.to_units(new_units);
        self.wave_height.to_units(new_units);
    }
}

impl fmt::Display for Swell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} @ {} {}", self.wave_height, self.period, self.direction)
    }
}