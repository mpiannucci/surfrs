//! A 1-D frequency spectrum built from ECMWF open-data wave parameters.
//!
//! ECMWF publishes no spectrum. It publishes total Hs, Tp, Tm-1,0, Tm02 and the
//! Hs in six period bands from 10 to 30 s. This module turns one point record
//! into an energy density curve (m²/Hz) on a fixed axis so it can be drawn next
//! to GFS and NDBC spectra:
//!
//! - **10–30 s:** one point per band at the band's midpoint period, with the
//!   band's energy spread evenly over its frequency width. These are exact
//!   averages of the model spectrum.
//! - **Below 10 s:** a JONSWAP shape fitted to what is left of Tm-1,0 and Tm02
//!   once the bands are taken out, scaled so its energy is the residual energy.
//!   When Tp < 10 s the peak is pinned to Tp ([`FitQuality::PeakAnchored`]);
//!   otherwise the peak comes from the moments alone ([`FitQuality::Approximate`]).
//!
//! The part below 10 s is a fit, not data. It cannot show two peaks below 10 s
//! (for example an 8–10 s swell under a shorter wind sea). See
//! `doc/ecmwf-wave-spec.md` for the method and how it scores against GFS spectra.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::units::Unit;

use super::{
    ecmwf_field::ECMWF_WAVE_PERIOD_BANDS, ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
};

/// Highest frequency of the ECWAM spectrum: 36 frequencies from 0.0345 Hz with
/// ratio 1.1, so 0.0345 × 1.1³⁵ Hz (≈ 1.03 s). The fitted shape below 10 s is
/// integrated from 0.1 Hz up to this frequency, matching the range ECWAM's own
/// moments come from.
pub const ECWAM_MAX_FREQUENCY: f64 = 0.969_534_07;

/// JONSWAP peak enhancements tried when fitting the part below 10 s.
pub const FIT_GAMMAS: [f64; 6] = [1.0, 1.5, 2.0, 3.3, 5.0, 7.0];

/// Peak enhancement used when only Tm-1,0 is known (AIFS).
pub const DEFAULT_GAMMA: f64 = 3.3;

/// Lower edge of the fitted part: the short edge of the 10–12 s band.
const FIT_MIN_FREQUENCY: f64 = 0.1;

/// GFS WW3 frequency grid: 0.035 × 1.07ⁿ Hz.
const GFS_FIRST_FREQUENCY: f64 = 0.035;
const GFS_FREQUENCY_RATIO: f64 = 1.07;
/// The GFS grid points used on the axis: n = 16 (0.103 Hz, 9.7 s) to
/// n = 39 (0.490 Hz, 2.04 s).
const GFS_AXIS_INDICES: std::ops::RangeInclusive<i32> = 16..=39;

/// Range of fitted peak frequencies when the peak is not pinned: 0.1 Hz (10 s)
/// to 0.6 Hz (1.7 s), in steps of 0.25%.
const FIT_PEAK_MIN: f64 = 0.1;
const FIT_PEAK_MAX: f64 = 0.6;
const FIT_PEAK_STEP_RATIO: f64 = 1.0025;

/// Simpson intervals (in ln f) used to integrate the JONSWAP shape.
const INTEGRATION_INTERVALS: usize = 400;

/// Frequencies of the spectrum axis in Hz, ascending (so periods descending):
/// six points at the band midpoint periods (27.5, 23, 19, 15.5, 13 and 11 s),
/// then the GFS WW3 frequencies from 0.103 Hz to 0.490 Hz. The axis is the same
/// for every record, so callers can send it once per source.
pub fn ecmwf_spectrum_frequencies() -> Vec<f64> {
    ecmwf_spectrum_periods().iter().map(|p| 1.0 / p).collect()
}

/// Periods of the spectrum axis in seconds, descending. See
/// [`ecmwf_spectrum_frequencies`].
pub fn ecmwf_spectrum_periods() -> Vec<f64> {
    let bands = ECMWF_WAVE_PERIOD_BANDS
        .iter()
        .rev()
        .map(|(min, max)| (*min as f64 + *max as f64) / 2.0);
    let gfs = GFS_AXIS_INDICES.map(|n| 1.0 / gfs_frequency(n));
    bands.chain(gfs).collect()
}

/// The frequency width (Hz) each axis point stands for, in axis order. Band
/// points get the full band width, GFS points the width of their WW3 bin
/// (edges at the geometric midpoints). `Σ density × width` is the energy on
/// the axis.
pub fn ecmwf_spectrum_bandwidths() -> Vec<f64> {
    let bands = ECMWF_WAVE_PERIOD_BANDS
        .iter()
        .rev()
        .map(|(min, max)| band_width(*min, *max));
    let half = GFS_FREQUENCY_RATIO.sqrt();
    let gfs = GFS_AXIS_INDICES.map(move |n| gfs_frequency(n) * (half - 1.0 / half));
    bands.chain(gfs).collect()
}

/// How the part of the spectrum below 10 s was placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitQuality {
    /// Tp < 10 s, so the peak of the fitted part is pinned at Tp and only the
    /// width (γ) is fitted. Against GFS and NDBC spectra the peak lands within
    /// a few tenths of a second.
    PeakAnchored,
    /// The peak comes from the residual mean periods alone (Tp ≥ 10 s, or no
    /// Tp, as in AIFS). The energy below 10 s is right, but where it peaks is
    /// a rough guess.
    Approximate,
}

/// The JONSWAP shape fitted to the energy below 10 s.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JonswapFit {
    /// s
    pub peak_period: f64,
    pub gamma: f64,
    /// Energy of the fitted part, m² (Hs²/16 of the energy below 10 s).
    pub energy: f64,
    /// Upper end of the range the shape is scaled over, Hz.
    pub max_frequency: f64,
    /// Multiplies the unit JONSWAP shape to give m²/Hz.
    pub scale: f64,
}

impl JonswapFit {
    /// Energy density in m²/Hz at `frequency`. Zero outside 0.1 Hz to
    /// `max_frequency`, the range the fit covers.
    pub fn density(&self, frequency: f64) -> f64 {
        if !(FIT_MIN_FREQUENCY..=self.max_frequency).contains(&frequency) {
            return 0.0;
        }
        self.scale * jonswap(frequency, 1.0 / self.peak_period, self.gamma)
    }

    /// Significant height of the fitted part, m.
    pub fn significant_height(&self) -> f64 {
        4.0 * self.energy.sqrt()
    }
}

/// ECMWF wave parameters at a point, in metres and seconds: what
/// [`ECMWFSpectrumFitter::fit`] needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFWaveParameters {
    /// Total Hs, m.
    pub significant_wave_height: f64,
    /// Tm-1,0, s.
    pub energy_period: f64,
    /// Tm02, s. None for AIFS.
    pub zero_crossing_period: Option<f64>,
    /// s. None for AIFS.
    pub peak_period: Option<f64>,
    /// Hs in each of [`ECMWF_WAVE_PERIOD_BANDS`], in that order (shortest
    /// periods first), m.
    pub band_heights: [f64; 6],
}

impl ECMWFWaveParameters {
    /// The parameters of a record, converted to metres. None unless Hs, Tm-1,0
    /// and all six bands are present.
    pub fn from_record(record: &ECMWFWavePointDataRecord) -> Option<Self> {
        let metres = |value: f64, unit: &Unit| unit.convert(value, &Unit::Meters);
        let hs = &record.significant_wave_height;

        let mut band_heights = [0.0; 6];
        for (slot, (min, max)) in band_heights.iter_mut().zip(ECMWF_WAVE_PERIOD_BANDS) {
            let band = record
                .period_bands
                .iter()
                .find(|b| b.min_period == min && b.max_period == max)?;
            *slot = metres(band.height.value?, &band.height.unit);
        }

        Some(ECMWFWaveParameters {
            significant_wave_height: metres(hs.value?, &hs.unit),
            energy_period: record.energy_period.value?,
            zero_crossing_period: record.zero_crossing_period.value,
            peak_period: record.peak_period.value,
            band_heights,
        })
    }
}

/// A 1-D spectrum on the axis from [`ecmwf_spectrum_frequencies`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFFittedSpectrum {
    /// Energy density in m²/Hz, one value per axis point (ascending frequency,
    /// descending period). The first six values are the band averages.
    pub density: Vec<f64>,
    /// How the part below 10 s was placed.
    pub fit: FitQuality,
    /// The shape fitted below 10 s. None when the curve is zero below 10 s:
    /// the bands hold all the energy, or the residual is too small for its
    /// moments to place it (see [`ECMWFSpectrumFitter::fit`]).
    pub short_period: Option<JonswapFit>,
}

/// Fits spectra for one model's frequency range. Building one integrates the
/// JONSWAP shape over a table of peaks and widths, so reuse it;
/// [`ECMWFSpectrumFitter::ecwam`] is a shared one for ECWAM.
pub struct ECMWFSpectrumFitter {
    max_frequency: f64,
    /// Every (peak frequency, γ) tried when the peak is free.
    table: Vec<ShapeMoments>,
}

#[derive(Clone, Copy, Debug)]
struct ShapeMoments {
    peak_frequency: f64,
    gamma: f64,
    /// ∫ shape df over [0.1, max_frequency]
    m0: f64,
    ln_tm10: f64,
    ln_tm02: f64,
}

impl ECMWFSpectrumFitter {
    /// A fitter whose fitted shape covers 0.1 Hz to `max_frequency`. Use the
    /// highest frequency of the model spectrum that the published moments were
    /// computed over.
    pub fn new(max_frequency: f64) -> Self {
        assert!(
            max_frequency > FIT_PEAK_MAX,
            "max_frequency must be above {FIT_PEAK_MAX} Hz"
        );
        let peak_count = ((FIT_PEAK_MAX / FIT_PEAK_MIN).ln() / FIT_PEAK_STEP_RATIO.ln()) as i32;
        let table = (0..=peak_count)
            .map(|i| FIT_PEAK_MIN * FIT_PEAK_STEP_RATIO.powi(i))
            .flat_map(|fp| FIT_GAMMAS.map(|gamma| shape_moments(fp, gamma, max_frequency)))
            .collect();
        ECMWFSpectrumFitter {
            max_frequency,
            table,
        }
    }

    /// The fitter for ECWAM (IFS HRES and ENS), up to [`ECWAM_MAX_FREQUENCY`].
    /// Built on first use and shared.
    pub fn ecwam() -> &'static Self {
        static FITTER: OnceLock<ECMWFSpectrumFitter> = OnceLock::new();
        FITTER.get_or_init(|| ECMWFSpectrumFitter::new(ECWAM_MAX_FREQUENCY))
    }

    pub fn max_frequency(&self) -> f64 {
        self.max_frequency
    }

    /// Build the spectrum for one set of parameters.
    ///
    /// 1. Total moments: m0 = Hs²/16, m₋₁ = Tm-1,0·m0, m2 = m0/Tm02².
    /// 2. Take out each band, assuming flat density d from f1 to f2:
    ///    m0 −= d(f2−f1), m₋₁ −= d·ln(f2/f1), m2 −= d(f2³−f1³)/3.
    /// 3. Fit JONSWAP (σ 0.07/0.09) to the residual log Tm-1,0 and log Tm02:
    ///    - Tp < 10 s: peak at Tp, γ from [`FIT_GAMMAS`] (`PeakAnchored`).
    ///    - Tp ≥ 10 s: peak and γ both fitted (`Approximate`).
    ///    - No Tm02 or no Tp (AIFS): peak fitted to Tm-1,0 with γ = 3.3
    ///      (`Approximate`).
    /// 4. Scale the shape so its energy from 0.1 Hz to `max_frequency` is the
    ///    residual m0.
    ///
    /// If the residual m0 is ≤ 0 (the bands hold everything, up to packing
    /// noise), the curve is zero below 10 s. A residual moment that is not
    /// positive is left out of the fit. If neither residual period is usable
    /// and Tp gives no anchor, there is nothing to place the residual with, so
    /// it is dropped and the curve is zero below 10 s too. That happens in
    /// pure swell (Tp ≈ 15 s), where the residual is a few cm of Hs and the
    /// flat-band assumption takes more than all of its m₋₁ and m2.
    ///
    /// None if Hs is missing or negative, or Tm-1,0 is not positive.
    pub fn fit(&self, parameters: &ECMWFWaveParameters) -> Option<ECMWFFittedSpectrum> {
        let hs = parameters.significant_wave_height;
        if !(hs.is_finite() && hs >= 0.0 && parameters.energy_period > 0.0) {
            return None;
        }

        // Band densities, and the residual moments.
        let mut m0 = hs * hs / 16.0;
        let mut m_1 = parameters.energy_period * m0;
        let mut m2 = parameters
            .zero_crossing_period
            .filter(|t| *t > 0.0)
            .map(|t| m0 / (t * t));
        let mut band_density = [0.0; 6];
        for (i, (min, max)) in ECMWF_WAVE_PERIOD_BANDS.iter().enumerate() {
            let (f1, f2) = (1.0 / *max as f64, 1.0 / *min as f64);
            let h = parameters.band_heights[i].max(0.0);
            let d = h * h / 16.0 / (f2 - f1);
            band_density[i] = d;
            m0 -= d * (f2 - f1);
            m_1 -= d * (f2 / f1).ln();
            if let Some(m2) = m2.as_mut() {
                *m2 -= d * (f2.powi(3) - f1.powi(3)) / 3.0;
            }
        }

        let anchor = parameters
            .peak_period
            .filter(|tp| *tp > 0.0 && *tp < 10.0)
            .map(|tp| 1.0 / tp);
        let fit = if anchor.is_some() {
            FitQuality::PeakAnchored
        } else {
            FitQuality::Approximate
        };

        let short_period = if m0 > 0.0 {
            let ln_tm10 = (m_1 > 0.0).then(|| (m_1 / m0).ln());
            let ln_tm02 = m2.filter(|m2| *m2 > 0.0).map(|m2| 0.5 * (m0 / m2).ln());
            self.fit_shape(anchor, ln_tm10, ln_tm02)
                .map(|shape| JonswapFit {
                    peak_period: 1.0 / shape.peak_frequency,
                    gamma: shape.gamma,
                    energy: m0,
                    max_frequency: self.max_frequency,
                    scale: m0 / shape.m0,
                })
        } else {
            None
        };

        // Bands are listed shortest period first; the axis runs long to short.
        let density = band_density
            .iter()
            .rev()
            .copied()
            .chain(GFS_AXIS_INDICES.map(|n| {
                short_period
                    .as_ref()
                    .map_or(0.0, |fit| fit.density(gfs_frequency(n)))
            }))
            .collect();

        Some(ECMWFFittedSpectrum {
            density,
            fit,
            short_period,
        })
    }

    fn fit_shape(
        &self,
        anchor: Option<f64>,
        ln_tm10: Option<f64>,
        ln_tm02: Option<f64>,
    ) -> Option<ShapeMoments> {
        let misfit = |s: &ShapeMoments| {
            ln_tm10.map_or(0.0, |t| (s.ln_tm10 - t).powi(2))
                + ln_tm02.map_or(0.0, |t| (s.ln_tm02 - t).powi(2))
        };
        let best = |candidates: &mut dyn Iterator<Item = ShapeMoments>| {
            candidates.min_by(|a, b| misfit(a).total_cmp(&misfit(b)))
        };

        match (anchor, ln_tm10, ln_tm02) {
            (Some(fp), ..) => best(
                &mut FIT_GAMMAS
                    .iter()
                    .map(|gamma| shape_moments(fp, *gamma, self.max_frequency)),
            ),
            (None, None, None) => None,
            // Both periods: fit the peak and γ.
            (None, Some(_), Some(_)) => best(&mut self.table.iter().copied()),
            // One period: two unknowns are too many, so fix γ.
            (None, ..) => best(
                &mut self
                    .table
                    .iter()
                    .copied()
                    .filter(|s| s.gamma == DEFAULT_GAMMA),
            ),
        }
    }
}

impl ECMWFWavePointDataRecord {
    /// The record as a 1-D frequency spectrum (m²/Hz) on the fixed axis from
    /// [`ecmwf_spectrum_frequencies`], fitted with
    /// [`ECMWFSpectrumFitter::ecwam`]. None unless Hs, Tm-1,0 and all six
    /// period bands are present.
    pub fn fitted_spectrum(&self) -> Option<ECMWFFittedSpectrum> {
        ECMWFSpectrumFitter::ecwam().fit(&ECMWFWaveParameters::from_record(self)?)
    }
}

fn gfs_frequency(n: i32) -> f64 {
    GFS_FIRST_FREQUENCY * GFS_FREQUENCY_RATIO.powi(n)
}

fn band_width(min_period: u8, max_period: u8) -> f64 {
    1.0 / min_period as f64 - 1.0 / max_period as f64
}

/// Unnormalised JONSWAP shape, σ = 0.07 at or below the peak and 0.09 above.
fn jonswap(f: f64, fp: f64, gamma: f64) -> f64 {
    let sigma = if f <= fp { 0.07 } else { 0.09 };
    let r = (-(f - fp).powi(2) / (2.0 * sigma * sigma * fp * fp)).exp();
    f.powi(-5) * (-1.25 * (fp / f).powi(4)).exp() * gamma.powf(r)
}

/// m0, ln Tm-1,0 and ln Tm02 of the JONSWAP shape over [0.1, max_frequency],
/// by Simpson's rule in ln f.
fn shape_moments(fp: f64, gamma: f64, max_frequency: f64) -> ShapeMoments {
    let (a, b) = (FIT_MIN_FREQUENCY.ln(), max_frequency.ln());
    let h = (b - a) / INTEGRATION_INTERVALS as f64;
    let (mut m0, mut m_1, mut m2) = (0.0, 0.0, 0.0);
    for i in 0..=INTEGRATION_INTERVALS {
        let weight = match i {
            0 => 1.0,
            i if i == INTEGRATION_INTERVALS => 1.0,
            i if i % 2 == 1 => 4.0,
            _ => 2.0,
        };
        let f = (a + h * i as f64).exp();
        // df = f d(ln f)
        let e = weight * jonswap(f, fp, gamma) * f;
        m0 += e;
        m_1 += e / f;
        m2 += e * f * f;
    }
    let scale = h / 3.0;
    let (m0, m_1, m2) = (m0 * scale, m_1 * scale, m2 * scale);
    ShapeMoments {
        peak_frequency: fp,
        gamma,
        m0,
        ln_tm10: (m_1 / m0).ln(),
        ln_tm02: 0.5 * (m0 / m2).ln(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecwam_max_frequency() {
        assert!((ECWAM_MAX_FREQUENCY - 0.0345 * 1.1f64.powi(35)).abs() < 1e-6);
    }

    #[test]
    fn test_axis() {
        let periods = ecmwf_spectrum_periods();
        assert_eq!(periods.len(), 30);
        assert_eq!(&periods[..6], &[27.5, 23.0, 19.0, 15.5, 13.0, 11.0]);
        assert!(periods.windows(2).all(|w| w[0] > w[1]));
        assert!(periods[6] < 10.0 && periods[6] > 9.5);
        assert!((periods[29] - 2.04).abs() < 0.01);
        assert_eq!(ecmwf_spectrum_bandwidths().len(), 30);
        assert_eq!(ecmwf_spectrum_frequencies().len(), 30);
    }

    #[test]
    fn test_shape_moments_are_ordered() {
        for gamma in FIT_GAMMAS {
            let s = shape_moments(0.15, gamma, ECWAM_MAX_FREQUENCY);
            // Tp > Tm-1,0 > Tm02 for a single-peaked shape.
            assert!(1.0 / 0.15 > s.ln_tm10.exp() && s.ln_tm10 > s.ln_tm02);
        }
        // A narrower peak has mean periods closer to Tp.
        let wide = shape_moments(0.15, 1.0, ECWAM_MAX_FREQUENCY);
        let narrow = shape_moments(0.15, 7.0, ECWAM_MAX_FREQUENCY);
        assert!(narrow.ln_tm02 > wide.ln_tm02);
    }
}
