//! A 1-D frequency spectrum built from ECMWF open-data wave parameters.
//!
//! ECMWF publishes no spectrum, only total Hs, Tp, Tm-1,0, Tm02 and the Hs in
//! six period bands from 10 to 30 s. From those:
//!
//! - **10–30 s:** each band's energy spread evenly over its frequency width.
//!   These are exact band averages of the model spectrum.
//! - **Below 10 s:** a JONSWAP shape fitted to what is left of Tm-1,0 and Tm02
//!   once the bands are taken out, scaled so its energy is the residual energy.
//!
//! The part below 10 s is a fit, not data: it cannot show two peaks below 10 s
//! (for example an 8–10 s swell under a shorter wind sea). See
//! `doc/ecmwf-wave-spec.md` for how it scores against GFS spectra.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::units::Unit;

use super::{
    ecmwf_field::ECMWF_WAVE_PERIOD_BANDS, ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
};

/// Highest frequency of the ECWAM spectrum: 36 frequencies from 0.0345 Hz with
/// ratio 1.1, so 0.0345 × 1.1³⁵ Hz.
pub const ECWAM_MAX_FREQUENCY: f64 = 0.969_534_07;

/// Lower edge of the fitted part: the short edge of the 10–12 s band.
const MIN_FREQUENCY: f64 = 0.1;
/// Peak enhancements tried, and the one used when there is too little to fit it.
const GAMMAS: [f64; 6] = [1.0, 1.5, 2.0, 3.3, 5.0, 7.0];
const DEFAULT_GAMMA: f64 = 3.3;
/// Peak frequencies tried when the peak is not pinned to Tp: 10 s to 1.7 s in
/// 1% steps. That is far finer than a plot axis (GFS points are 7% apart).
const PEAK_RANGE: (f64, f64) = (0.1, 0.6);
const PEAK_STEP: f64 = 1.01;

/// How the part of the spectrum below 10 s was placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitQuality {
    /// Tp < 10 s, so the peak is pinned at Tp and only γ is fitted.
    PeakAnchored,
    /// The peak comes from the residual mean periods alone (Tp ≥ 10 s, or no
    /// Tp as in AIFS). The energy below 10 s is right; where it peaks is a
    /// rough guess.
    Approximate,
}

/// The JONSWAP shape fitted below 10 s.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JonswapFit {
    /// s
    pub peak_period: f64,
    pub gamma: f64,
    /// Energy below 10 s, m² (Hs²/16 of the residual).
    pub energy: f64,
    /// Multiplies the unit JONSWAP shape to give m²/Hz.
    pub scale: f64,
}

/// ECMWF wave parameters at a point, in metres and seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFWaveParameters {
    pub significant_wave_height: f64,
    /// Tm-1,0
    pub energy_period: f64,
    /// Tm02. None for AIFS.
    pub zero_crossing_period: Option<f64>,
    /// None for AIFS.
    pub peak_period: Option<f64>,
    /// Hs in each of [`ECMWF_WAVE_PERIOD_BANDS`], in that order.
    pub band_heights: [f64; 6],
}

/// A 1-D spectrum: band averages from 10 to 30 s and a fitted shape below 10 s.
/// Sample it at any frequencies with [`ECMWFFittedSpectrum::density`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFFittedSpectrum {
    /// m²/Hz in each of [`ECMWF_WAVE_PERIOD_BANDS`], in that order.
    pub band_density: [f64; 6],
    pub fit: FitQuality,
    /// None when the curve is zero below 10 s: the bands hold all the energy,
    /// or the residual is too small for its moments to place it.
    pub short_period: Option<JonswapFit>,
    /// Upper end of the fitted part, Hz.
    pub max_frequency: f64,
}

impl ECMWFFittedSpectrum {
    /// Energy density in m²/Hz. Below 0.1 Hz this is the average of the band
    /// holding `frequency` (0 outside 10–30 s); from 0.1 Hz to `max_frequency`
    /// it is the fitted shape; above that it is 0.
    pub fn density(&self, frequency: f64) -> f64 {
        if frequency < MIN_FREQUENCY {
            return ECMWF_WAVE_PERIOD_BANDS
                .iter()
                .zip(self.band_density)
                .find(|((min, max), _)| (1.0 / *max as f64..1.0 / *min as f64).contains(&frequency))
                .map_or(0.0, |(_, density)| density);
        }
        match &self.short_period {
            Some(fit) if frequency <= self.max_frequency => {
                fit.scale * jonswap(frequency, 1.0 / fit.peak_period, fit.gamma)
            }
            _ => 0.0,
        }
    }
}

impl ECMWFWaveParameters {
    /// The parameters of a record, in metres. None unless Hs, Tm-1,0 and all
    /// six bands are present.
    pub fn from_record(record: &ECMWFWavePointDataRecord) -> Option<Self> {
        let metres = |value: f64, unit: &Unit| unit.convert(value, &Unit::Meters);
        let mut band_heights = [0.0; 6];
        for (slot, (min, max)) in band_heights.iter_mut().zip(ECMWF_WAVE_PERIOD_BANDS) {
            let band = record
                .period_bands
                .iter()
                .find(|b| (b.min_period, b.max_period) == (min, max))?;
            *slot = metres(band.height.value?, &band.height.unit);
        }
        let hs = &record.significant_wave_height;
        Some(ECMWFWaveParameters {
            significant_wave_height: metres(hs.value?, &hs.unit),
            energy_period: record.energy_period.value?,
            zero_crossing_period: record.zero_crossing_period.value,
            peak_period: record.peak_period.value,
            band_heights,
        })
    }

    /// Build the spectrum, fitting the part below 10 s over 0.1 Hz to
    /// `max_frequency` (the top of the model spectrum the published moments
    /// come from; [`ECWAM_MAX_FREQUENCY`] for IFS).
    ///
    /// 1. Total moments: m0 = Hs²/16, m₋₁ = Tm-1,0·m0, m2 = m0/Tm02².
    /// 2. Take out each band, assuming flat density d from f1 to f2:
    ///    m0 −= d(f2−f1), m₋₁ −= d·ln(f2/f1), m2 −= d(f2³−f1³)/3.
    /// 3. Fit JONSWAP (σ 0.07/0.09) to the residual log Tm-1,0 and log Tm02:
    ///    - Tp < 10 s: peak at Tp, γ fitted (`PeakAnchored`).
    ///    - Tp ≥ 10 s: peak and γ fitted (`Approximate`).
    ///    - Only Tm-1,0 (AIFS): peak fitted with γ = 3.3 (`Approximate`).
    /// 4. Scale the shape so its energy over the fitted range is the residual m0.
    ///
    /// The curve is zero below 10 s if the residual m0 is ≤ 0 (packing noise),
    /// or if neither residual period is positive and there is no Tp to anchor
    /// to (pure swell, where the residual is a few cm of Hs).
    ///
    /// None if Hs is negative or Tm-1,0 is not positive.
    pub fn fitted_spectrum(&self, max_frequency: f64) -> Option<ECMWFFittedSpectrum> {
        let hs = self.significant_wave_height;
        if !(hs >= 0.0 && self.energy_period > 0.0) {
            return None;
        }

        let mut m0 = hs * hs / 16.0;
        let mut m_1 = self.energy_period * m0;
        let mut m2 = self
            .zero_crossing_period
            .filter(|t| *t > 0.0)
            .map(|t| m0 / (t * t));
        let mut band_density = [0.0; 6];
        for (d, ((min, max), h)) in band_density
            .iter_mut()
            .zip(ECMWF_WAVE_PERIOD_BANDS.iter().zip(self.band_heights))
        {
            let (f1, f2) = (1.0 / *max as f64, 1.0 / *min as f64);
            *d = h.max(0.0).powi(2) / 16.0 / (f2 - f1);
            m0 -= *d * (f2 - f1);
            m_1 -= *d * (f2 / f1).ln();
            if let Some(m2) = m2.as_mut() {
                *m2 -= *d * (f2.powi(3) - f1.powi(3)) / 3.0;
            }
        }

        let anchor = self
            .peak_period
            .filter(|tp| *tp > 0.0 && *tp < 10.0)
            .map(|tp| 1.0 / tp);
        let short_period = if m0 > 0.0 {
            let ln_tm10 = (m_1 > 0.0).then(|| (m_1 / m0).ln());
            let ln_tm02 = m2.filter(|m2| *m2 > 0.0).map(|m2| 0.5 * (m0 / m2).ln());
            fit_jonswap(anchor, ln_tm10, ln_tm02, max_frequency).map(|(fp, gamma, shape_m0)| {
                JonswapFit {
                    peak_period: 1.0 / fp,
                    gamma,
                    energy: m0,
                    scale: m0 / shape_m0,
                }
            })
        } else {
            None
        };

        Some(ECMWFFittedSpectrum {
            band_density,
            fit: match anchor {
                Some(_) => FitQuality::PeakAnchored,
                None => FitQuality::Approximate,
            },
            short_period,
            max_frequency,
        })
    }
}

impl ECMWFWavePointDataRecord {
    /// The record as a 1-D spectrum, fitted up to [`ECWAM_MAX_FREQUENCY`]. None
    /// unless Hs, Tm-1,0 and all six period bands are present.
    pub fn fitted_spectrum(&self) -> Option<ECMWFFittedSpectrum> {
        ECMWFWaveParameters::from_record(self)?.fitted_spectrum(ECWAM_MAX_FREQUENCY)
    }
}

/// Peak frequency, γ and ∫shape df of the JONSWAP whose log Tm-1,0 and Tm02
/// over [0.1, max_frequency] best match the targets that are present.
fn fit_jonswap(
    anchor: Option<f64>,
    ln_tm10: Option<f64>,
    ln_tm02: Option<f64>,
    max_frequency: f64,
) -> Option<(f64, f64, f64)> {
    let targets = ln_tm10.is_some() as usize + ln_tm02.is_some() as usize;
    if anchor.is_none() && targets == 0 {
        return None;
    }
    // γ is only fitted when there are more targets than other unknowns.
    let gammas: &[f64] = if targets > anchor.is_none() as usize {
        &GAMMAS
    } else {
        &[DEFAULT_GAMMA]
    };
    let misfit = |s: &Shape| {
        ln_tm10.map_or(0.0, |t| (s.ln_tm10 - t).powi(2))
            + ln_tm02.map_or(0.0, |t| (s.ln_tm02 - t).powi(2))
    };
    let best = |shapes: &mut dyn Iterator<Item = Shape>| {
        shapes
            .min_by(|a, b| misfit(a).total_cmp(&misfit(b)))
            .map(|s| (s.peak_frequency, s.gamma, s.m0))
    };

    match anchor {
        // Pinned peak: only the γ values to try, computed directly.
        Some(fp) => best(&mut gammas.iter().map(|g| Shape::new(fp, *g, max_frequency))),
        None => best(
            &mut shape_table(max_frequency)
                .iter()
                .filter(|s| gammas.contains(&s.gamma))
                .copied(),
        ),
    }
}

/// A JONSWAP shape and its moments over [0.1, max_frequency].
#[derive(Clone, Copy, Debug)]
struct Shape {
    peak_frequency: f64,
    gamma: f64,
    /// ∫ shape df
    m0: f64,
    ln_tm10: f64,
    ln_tm02: f64,
}

impl Shape {
    fn new(peak_frequency: f64, gamma: f64, max_frequency: f64) -> Self {
        let (m0, ln_tm10, ln_tm02) = shape_moments(peak_frequency, gamma, max_frequency);
        Shape {
            peak_frequency,
            gamma,
            m0,
            ln_tm10,
            ln_tm02,
        }
    }
}

/// Every (peak frequency, γ) tried when the peak is free, for one
/// `max_frequency`. Built on first use (a few ms) and kept.
fn shape_table(max_frequency: f64) -> Arc<Vec<Shape>> {
    static TABLES: Mutex<Vec<(u64, Arc<Vec<Shape>>)>> = Mutex::new(Vec::new());
    let mut tables = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    let key = max_frequency.to_bits();
    if let Some((_, table)) = tables.iter().find(|(k, _)| *k == key) {
        return table.clone();
    }
    let steps = ((PEAK_RANGE.1 / PEAK_RANGE.0).ln() / PEAK_STEP.ln()).ceil() as i32;
    let table: Arc<Vec<Shape>> = Arc::new(
        (0..=steps)
            .map(|i| PEAK_RANGE.0 * PEAK_STEP.powi(i))
            .flat_map(|fp| GAMMAS.map(|gamma| Shape::new(fp, gamma, max_frequency)))
            .collect(),
    );
    tables.push((key, table.clone()));
    table
}

/// Unnormalised JONSWAP shape, σ = 0.07 at or below the peak and 0.09 above.
fn jonswap(f: f64, fp: f64, gamma: f64) -> f64 {
    let sigma = if f <= fp { 0.07 } else { 0.09 };
    let r = (-(f - fp).powi(2) / (2.0 * sigma * sigma * fp * fp)).exp();
    f.powi(-5) * (-1.25 * (fp / f).powi(4)).exp() * gamma.powf(r)
}

/// m0, ln Tm-1,0 and ln Tm02 of the JONSWAP shape over [0.1, max_frequency],
/// by Simpson's rule in ln f.
fn shape_moments(fp: f64, gamma: f64, max_frequency: f64) -> (f64, f64, f64) {
    const N: usize = 200;
    let (a, b) = (MIN_FREQUENCY.ln(), max_frequency.ln());
    let h = (b - a) / N as f64;
    let (mut m0, mut m_1, mut m2) = (0.0, 0.0, 0.0);
    for i in 0..=N {
        let weight = if i == 0 || i == N {
            1.0
        } else {
            (2 + 2 * (i % 2)) as f64
        };
        let f = (a + h * i as f64).exp();
        let e = weight * jonswap(f, fp, gamma) * f; // df = f d(ln f)
        m0 += e;
        m_1 += e / f;
        m2 += e * f * f;
    }
    (m0 * h / 3.0, (m_1 / m0).ln(), 0.5 * (m0 / m2).ln())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecwam_max_frequency() {
        assert!((ECWAM_MAX_FREQUENCY - 0.0345 * 1.1f64.powi(35)).abs() < 1e-6);
    }

    #[test]
    fn test_shape_moments() {
        // Tp > Tm-1,0 > Tm02 for a single peak, and a narrower peak has mean
        // periods closer to Tp.
        let (_, tm10, tm02) = shape_moments(0.15, 3.3, ECWAM_MAX_FREQUENCY);
        assert!(1.0 / 0.15 > tm10.exp() && tm10 > tm02);
        let wide = shape_moments(0.15, 1.0, ECWAM_MAX_FREQUENCY).2;
        let narrow = shape_moments(0.15, 7.0, ECWAM_MAX_FREQUENCY).2;
        assert!(narrow > wide);
    }

    #[test]
    fn test_fit_recovers_a_jonswap() {
        // The moments of a known shape are matched by that shape.
        let (_, tm10, tm02) = shape_moments(0.2, 3.3, ECWAM_MAX_FREQUENCY);
        let (fp, gamma, _) =
            fit_jonswap(None, Some(tm10), Some(tm02), ECWAM_MAX_FREQUENCY).unwrap();
        // To the 1% step of the peak frequencies tried.
        assert!(
            (fp / 0.2 - 1.0).abs() < 0.005 && gamma == 3.3,
            "{fp} {gamma}"
        );
    }
}
