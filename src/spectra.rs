use serde::{Serialize, Deserialize};

use crate::{tools::{vector::diff, analysis::{WatershedError, watershed}, waves::pt_mean}, swell::{SwellProviderError, SwellSummary}, units::direction::DirectionConvention};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpectralAxis {
    Frequency, 
    Direction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spectra {
    /// Frequency bins in hz
    pub frequency: Vec<f64>, 
    /// Direction bins in rad
    pub direction: Vec<f64>, 
    /// Energy values in m2/hz/rad
    pub energy: Vec<f64>,
}

impl Spectra {
    pub fn new(frequency: Vec<f64>, direction: Vec<f64>, values: Vec<f64>) -> Self {
        Spectra {
            frequency, 
            direction, 
            energy: values,
        }
    }

    /// Number of frequency bins
    pub fn nk(&self) -> usize {
        self.frequency.len()
    }

    /// Vector of frequency bandwidths
    pub fn dk(&self) -> Vec<f64> {
        diff(&self.frequency)
    }

    /// Number of directional bins
    pub fn nth(&self) -> usize {
        self.direction.len()
    }

    /// Vector of directional bandwidths
    pub fn dth(&self) -> Vec<f64> {
        diff(&self.direction)
    }

    /// One dimensional representation of the energy across the given axis
    /// Result is in m2/hz for SpectralAxis::Frequency or m2/rad for SpectralAxis::Direction
    pub fn oned(&self, axis: SpectralAxis) -> Vec<f64> {
        let nk = self.nk();
        let nth = self.nth();

        match axis {
            SpectralAxis::Frequency => {
                let dth = self.dth();

                let mut oned = vec![0.0; nk];
                for ik in 0..nk {
                    for ith in 0..nth {
                        let i = ik + (ith * nk);
                        oned[ik] += dth[ith] * self.energy[i];
                    }
                }
                oned
            }, 
            SpectralAxis::Direction => {
                let dk = self.dk(); 

                let mut oned = vec![0.0; nth];
                for ith in 0..nth {
                    for ik in 0..nk {
                        let i = ik + (ith * nk);
                        oned[ith] += dk[ik] * self.energy[i];
                    }
                }

                oned
            }
        }
    }

    /// Partition the energy data into discrete swell components
    pub fn partition(&self, levels: usize) -> Result<(Vec<i32>, usize), WatershedError> {
        watershed(&self.energy, self.frequency.len(), self.direction.len(), levels)
    }

    /// Extract swell components
    pub fn swell_data(&self, depth: Option<f64>, wind_speed: Option<f64>, wind_direction: Option<f64>, source_direction_convention: DirectionConvention) -> Result<crate::swell::SwellSummary, SwellProviderError> {
        let (imo, partition_count) = match watershed(
            &self.energy,
            self.frequency.len(),
            self.direction.len(),
            100,
        ) {
            Ok(result) => Ok(result), 
            Err(_) => Err(SwellProviderError::InsufficientData("watershed segmentation of the spectra failed".into())),
        }?;

        let (summary, components) = pt_mean(
            partition_count, 
            &imo, 
            &self.frequency, 
            &self.direction,
            &self.energy, 
            &self.dk(),
            &self.dth(),
            depth, 
            wind_speed, 
            wind_direction, 
            source_direction_convention
        );

        Ok(SwellSummary {
            summary, 
            components,
        })
    }
}
