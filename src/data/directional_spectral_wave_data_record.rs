use std::f64::consts::PI;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    spectra::Spectra,
    swell::{SwellProvider, SwellProviderError, SwellSummary},
    tools::math::f_eq,
    units::direction,
};

use super::spectral_wave_data_record::SpectralWaveDataRecord;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DirectionalSpectralWaveDataRecord {
    pub date: DateTime<Utc>,
    pub spectra: Spectra,
}

impl DirectionalSpectralWaveDataRecord {
    pub fn new(
        date: &DateTime<Utc>,
        direction: &[f64],
        frequency: &[f64],
        energy_spectra: &[f64],
        mean_wave_direction: &[f64],
        primary_wave_direction: &[f64],
        first_polar_coefficient: &[f64],
        second_polar_coefficient: &[f64],
    ) -> Self {
        let mut directional_spectra = vec![0.0; frequency.len() * direction.len()];

        for (ik, _) in frequency.iter().enumerate() {
            for (ith, angle) in direction.iter().enumerate() {
                if f_eq(energy_spectra[ik], 999.9)
                    || f_eq(mean_wave_direction[ik], 999.0)
                    || f_eq(primary_wave_direction[ik], 999.0)
                    || f_eq(first_polar_coefficient[ik], 999.0)
                    || f_eq(second_polar_coefficient[ik], 999.0)
                {
                    continue;
                }

                let i = ik + (ith * frequency.len());

                let first = first_polar_coefficient[ik]
                    * (angle - mean_wave_direction[ik].to_radians()).cos();
                let second = second_polar_coefficient[ik]
                    * (2.0 * (angle - primary_wave_direction[ik].to_radians())).cos();

                let v = energy_spectra[ik] * (1.0 / PI) * (0.5 + first + second);
                directional_spectra[i] = if v >= 0.0 { v } else { 0.0 };
            }
        }

        let spectra = Spectra::new(
            frequency.to_vec(),
            direction.to_vec(),
            directional_spectra,
            direction::DirectionConvention::From,
        );

        DirectionalSpectralWaveDataRecord {
            date: date.clone(),
            spectra,
        }
    }

    pub fn from_data_records(
        direction: &[f64],
        energy_spectra: SpectralWaveDataRecord,
        mean_wave_direction: SpectralWaveDataRecord,
        primary_wave_direction: SpectralWaveDataRecord,
        first_polar_coefficient: SpectralWaveDataRecord,
        second_polar_coefficient: SpectralWaveDataRecord,
    ) -> Self {
        Self::new(
            &energy_spectra.date,
            direction,
            &energy_spectra.frequency,
            &energy_spectra.value,
            &mean_wave_direction.value,
            &primary_wave_direction.value,
            &first_polar_coefficient.value,
            &second_polar_coefficient.value,
        )
    }
}

impl SwellProvider for DirectionalSpectralWaveDataRecord {
    fn swell_data(&self) -> Result<SwellSummary, SwellProviderError> {
        self.spectra.swell_data(None, None, None, Some(0.8))
        // ?;

        // swell_data.summary.direction.value.as_mut().unwrap().flip();

        // swell_data.components
        //     .iter_mut()
        //     .for_each(|s| s.direction.value.as_mut().unwrap().flip());

        // Ok(swell_data)
    }
}
