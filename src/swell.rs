use serde::{Deserialize, Serialize};

use crate::dimensional_data::DimensionalData;
use crate::units::{Direction, UnitConvertible, Unit, UnitSystem};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Swell {
    pub wave_height: DimensionalData<f64>,
    pub period: DimensionalData<f64>,
    pub direction: DimensionalData<Direction>,
    pub energy: Option<DimensionalData<f64>>,
    pub partition: Option<usize>,
}

impl Swell {
    pub fn new(
        units: &UnitSystem,
        wave_height: f64,
        period: f64,
        direction: Direction,
        energy: Option<f64>,
        partition: Option<usize>,
    ) -> Swell {
        Swell {
            wave_height: DimensionalData {
                value: Some(wave_height),
                variable_name: "wave height".into(),
                unit: match units {
                    UnitSystem::Metric => Unit::Meters,
                    UnitSystem::English => Unit::Feet,
                    _ => Unit::Unknown,
                },
            },
            period: DimensionalData {
                value: Some(period),
                variable_name: "period".into(),
                unit: Unit::Seconds,
            },
            direction: DimensionalData {
                value: Some(direction),
                variable_name: "direction".into(),
                unit: Unit::Degrees,
            },
            energy: energy.map(|v| DimensionalData {
                value: Some(v),
                variable_name: "energy".into(),
                unit: Unit::MetersSquaredPerHertz,
            }),
            partition,
        }
    }
}

impl UnitConvertible for Swell {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.wave_height.to_units(new_units);
        self
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
    SwellPartitionError(String),
}

impl Display for SwellProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::NotImplemented => "not implemented".into(),
            Self::InsufficientData(s) => format!("insufficient data to create swell:  {s}"),
            Self::SwellPartitionError(s) => format!("error partitioning swell: {s}"),
        };
        write!(f, "{}", description)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Returns a filtered vector of components that has removed the probably false components
    pub fn filtered_components(&self) -> Vec<Swell> {
        let false_components = self.probable_false_components();
        self.components
            .iter()
            .enumerate()
            .filter(|(i, _)| !false_components.contains(i))
            .map(|(_, s)| s.clone())
            .collect()
    }
}

impl UnitConvertible for SwellSummary {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.summary.to_units(new_units);
        self.components.iter_mut().for_each(|c| {c.to_units(new_units);});
        self
    }
}
