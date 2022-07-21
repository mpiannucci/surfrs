use serde::{Serialize, Deserialize};

use crate::dimensional_data::DimensionalData;
use crate::tools::zero_spectral_moment;
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

    pub fn from_spectra(frequency: &[f64], energy: &[f64], direction: &[Direction]) -> Result<Self, SwellProviderError> {
        let mut max_energy: Option<(usize, f64)> = None;
        let mut zero_moment = 0.0f64;

        for (i, freq) in frequency.iter().enumerate() {
            let bandwidth = if i > 0 {
                (freq - frequency[i-1]).abs()
            } else {
                (frequency[i+1] - freq).abs()
            };

            zero_moment += zero_spectral_moment(energy[i], bandwidth);

            if let Some(current_max_energy) = max_energy {
                if energy[i] > current_max_energy.1 {
                    max_energy = Some((i, energy[i]));
                }
            } else {
                max_energy = Some((i, energy[i]));
            }
        }

        match max_energy {
            Some((max_energy_index, energy)) => {
                let wave_height = 4.0 * zero_moment.sqrt();
                let period = 1.0 / frequency[max_energy_index];
                let direction = direction[max_energy_index].clone();
                Ok(Swell::new(&Units::Metric, wave_height, period, direction, Some(energy)))
            }, 
            None => Err(SwellProviderError::InsufficientData("Failed to extract the max energy frequency".to_string()))
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

pub trait SwellProvider {
    fn wave_summary(&self) -> Result<Swell, SwellProviderError>;
    fn swell_components(&self) -> Result<Vec<Swell>, SwellProviderError>;
}