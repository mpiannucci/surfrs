use serde::{Deserialize, Serialize};

use crate::dimensional_data::DimensionalData;
use crate::units::{Direction, Measurement, UnitConvertible, Units};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Swell {
    pub wave_height: DimensionalData<f64>,
    pub period: DimensionalData<f64>,
    pub direction: DimensionalData<Direction>,
    pub energy: Option<DimensionalData<f64>>,
}

impl Swell {
    pub fn new(
        units: &Units,
        wave_height: f64,
        period: f64,
        direction: Direction,
        energy: Option<f64>,
    ) -> Swell {
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
        write!(
            f,
            "{} @ {} {}",
            self.wave_height, self.period, self.direction
        )
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
            Self::InsufficientData(s) => format!("insufficient data to create swell:  {s}"),
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

impl SwellSummary {
    /// Extracts the component indexes which match swell components that may show up only because of a 
    /// mirrored false positive from spectral extraction. This usally happens at the exact same dominant periods, with about
    /// a 180 degree difference in mean wave direction
    pub fn probable_false_components(&self) -> Vec<usize> {
        let mut component_periods: HashMap<String, Vec<usize>> = HashMap::new();
        self.components.iter().enumerate().for_each(|(i, c)| {
            let key = c.period.to_string();
            match component_periods.get_mut(&key) {
                Some(v) => v.push(i),
                None => {
                    component_periods.insert(key, vec![i]);
                }
            };
        });

        let mut indexes = Vec::new();
        for (_, i_components) in &component_periods {
            if i_components.len() < 2 {
                continue;
            }

            let truth_direction = self.components[i_components[0]].direction.value.as_ref().unwrap();

            // Assuming sorted from max to min energy already 
            for i in 1..i_components.len() {
                let idx = i_components[i];
                if truth_direction.is_opposite(self.components[idx].direction.value.as_ref().unwrap()) {
                    indexes.push(idx);
                }
            }
        }

        indexes
    }
}
