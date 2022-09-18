use crate::{tools::{vector::diff, analysis::{WatershedError, watershed}, waves::pt_mean}, swell::{SwellProvider, SwellProviderError, SwellSummary}};


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

    /// One dimensional representation of the energy across the frequency axis
    /// Result is in m2/hz
    pub fn oned(&self) -> Vec<f64> {
        let dth = self.dth();
        let nk = self.nk();
        let nth = self.nth();

        let mut oned = vec![0.0; nk];
        for ik in 0..nk {
            for ith in 0..nth {
                let i = ik + (ith * nk);
                oned[ik] += dth[ith] * self.energy[i];
            }
        }
        oned
    }

    /// Partition the energy data into discrete swell components
    pub fn partition(&self, levels: usize) -> Result<(Vec<i32>, usize), WatershedError> {
        watershed(&self.energy, self.frequency.len(), self.direction.len(), levels)
    }

    /// Extract swell components
    pub fn swell_data(&self, depth: Option<f64>, wind_speed: Option<f64>, wind_direction: Option<f64>) -> Result<crate::swell::SwellSummary, SwellProviderError> {
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
            depth, 
            wind_speed, 
            wind_direction, 
        );

        Ok(SwellSummary {
            summary, 
            components,
        })
    }
}
