use serde::{Serialize, Deserialize};

use crate::dimensional_data::DimensionalData;
use crate::units::{Units, Measurement, Direction, UnitConvertible};
use std::fmt::{self, Display, Debug};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Swell {
    pub wave_height: DimensionalData<f64>,
    pub period: DimensionalData<f64>,
    pub direction: DimensionalData<Direction>,
    pub energy: Option<DimensionalData<f64>>,
}

impl Swell {
    pub fn new(units: &Units, wave_height: f64, period: f64, direction: Direction, energy: Option<f64>) -> Swell {
        Swell {
            wave_height: DimensionalData {
                value: Some(wave_height),
                variable_name: "wave height".into(),
                measurement: Measurement::Length,
                unit: units.clone(),
            },
            period: DimensionalData {
                value: Some(period),
                variable_name: "period".into(),
                measurement: Measurement::Time,
                unit: units.clone(),
            },
            direction: DimensionalData {
                value: Some(direction),
                variable_name: "direction".into(),
                measurement: Measurement::Direction,
                unit: units.clone(),
            }, 
            energy: energy.map(|v| DimensionalData {
                value: Some(v),
                variable_name: "energy".into(),
                measurement: Measurement::WaveEnergy,
                unit: units.clone(),
            }),
        }
    }
}

impl UnitConvertible<Swell> for Swell {
    fn to_units(&mut self, new_units: &Units) {
        self.wave_height.to_units(new_units);
    }
}

impl fmt::Display for Swell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} @ {} {}", self.wave_height, self.period, self.direction)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwellProviderError {
    NotImplemented, 
    InsufficientData(String),
}

impl Display for SwellProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::NotImplemented => "not implemented".into(), 
            Self::InsufficientData(s) => format!("insufficient data to create swell:  {s}")
        };
        write!(f, "{}", description)
    }
}

pub struct SwellSummary {
    pub summary: Swell, 
    pub components: Vec<Swell>,
}

pub trait SwellProvider {
    fn swell_data(&self) -> Result<SwellSummary, SwellProviderError>;
}