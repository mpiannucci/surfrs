//! Scores the ECMWF fitted spectrum against GFS-Wave point spectra.
//!
//! For each GFS spectrum, compute what ECMWF would publish (Hs, Tp, Tm-1,0,
//! Tm02 and the six 10–30 s band heights), rebuild the curve with
//! `ECMWFSpectrumFitter`, and compare the part below 10 s with the true
//! spectrum. GFS spectra go to 0.964 Hz, close to ECWAM's 0.97 Hz, so the
//! published moments see the same high-frequency tail ECWAM's do.
//!
//! ```sh
//! cargo run --release --example validate_ecmwf_spectrum_fit [path/to/gfswave.spec ...]
//! ```
//!
//! Defaults to `mock/gfswave.44097.spec`.

use surfrs::{
    data::{
        ecmwf_field::ECMWF_WAVE_PERIOD_BANDS,
        ecmwf_fitted_spectrum::{
            ecmwf_spectrum_bandwidths, ECMWFSpectrumFitter, ECMWFWaveParameters, FitQuality,
            JonswapFit,
        },
        forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecordCollection,
    },
    spectra::SpectralAxis,
};

struct Case {
    label: String,
    tp: f64,
    hs: f64,
    /// Peaks below 10 s in the true spectrum (local maxima above 25% of the max).
    short_peaks: Vec<f64>,
    ifs: Score,
    aifs: Score,
    /// Hs from integrating the curve over the axis, over the published Hs.
    axis_hs_ratio: f64,
}

struct Score {
    fit: FitQuality,
    /// |fitted − true| peak period below 10 s, s. None when nothing is fitted.
    peak_error: Option<f64>,
    fitted_peak: Option<f64>,
    true_peak: f64,
    /// Σ|fit − truth|·df / Σ truth·df over 0.1 Hz to the top of the spectrum.
    /// 0 is perfect, 2 is disjoint.
    shape_error: f64,
    /// The same, only up to 0.49 Hz (the axis, and about NDBC's 0.485 Hz).
    shape_error_axis: f64,
    gamma: Option<f64>,
    /// RMS of the log misfit between the fitted shape's Tm-1,0 / Tm02 and the
    /// residual targets (only the targets the fit used).
    moment_misfit: Option<f64>,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let paths = if paths.is_empty() {
        vec!["mock/gfswave.44097.spec".to_string()]
    } else {
        paths
    };

    let mut fitter: Option<(f64, ECMWFSpectrumFitter)> = None;
    let mut cases = Vec::new();
    let mut hs_check: Vec<f64> = Vec::new();

    for path in &paths {
        let raw = std::fs::read_to_string(path).expect("spec file");
        let mut collection = ForecastSpectralWaveDataRecordCollection::from_data(&raw);
        let (_, records) = collection.records().expect("spec records");
        for record in records {
            let f = record.spectra.frequency.clone();
            let s = record.spectra.oned(SpectralAxis::Frequency);
            let (lo, hi) = geometric_edges(&f);
            let df: Vec<f64> = lo.iter().zip(&hi).map(|(a, b)| b - a).collect();
            let top = *hi.last().unwrap();

            let moment = |n: i32| -> f64 {
                f.iter()
                    .zip(&s)
                    .zip(&df)
                    .map(|((f, s), df)| s * f.powi(n) * df)
                    .sum()
            };
            let (m0, m_1, m2) = (moment(0), moment(-1), moment(2));
            if m0 <= 0.0 {
                continue;
            }
            let hs = 4.0 * m0.sqrt();
            let tp = 1.0 / f[argmax(&s)];
            let mut band_heights = [0.0; 6];
            for (slot, (min, max)) in band_heights.iter_mut().zip(ECMWF_WAVE_PERIOD_BANDS) {
                let (f1, f2) = (1.0 / max as f64, 1.0 / min as f64);
                let e: f64 = (0..f.len())
                    .map(|k| s[k] * (hi[k].min(f2) - lo[k].max(f1)).max(0.0))
                    .sum();
                *slot = 4.0 * e.sqrt();
            }
            let ifs = ECMWFWaveParameters {
                significant_wave_height: hs,
                energy_period: m_1 / m0,
                zero_crossing_period: Some((m0 / m2).sqrt()),
                peak_period: Some(tp),
                band_heights,
            };
            let aifs = ECMWFWaveParameters {
                zero_crossing_period: None,
                peak_period: None,
                ..ifs.clone()
            };

            if fitter.as_ref().map(|(t, _)| *t) != Some(top) {
                fitter = Some((top, ECMWFSpectrumFitter::new(top)));
            }
            let fitter = &fitter.as_ref().unwrap().1;

            // Truth below 10 s: the bins whose centre is above 0.1 Hz.
            let short: Vec<usize> = (0..f.len()).filter(|k| f[*k] > 0.1).collect();
            let short_s: Vec<f64> = short.iter().map(|k| s[*k]).collect();
            if short_s.iter().sum::<f64>() <= 0.0 {
                continue;
            }
            let true_peak = 1.0 / f[short[argmax(&short_s)]];
            let max_short = short_s.iter().cloned().fold(0.0, f64::max);
            let short_peaks: Vec<f64> = (1..short.len() - 1)
                .filter(|i| {
                    short_s[*i] >= short_s[i - 1]
                        && short_s[*i] > short_s[i + 1]
                        && short_s[*i] > 0.25 * max_short
                })
                .map(|i| 1.0 / f[short[i]])
                .collect();

            let score = |p: &ECMWFWaveParameters| -> Option<Score> {
                let spectrum = fitter.fit(p)?;
                let fit = spectrum.short_period;
                let moment_misfit = fit.map(|fit| {
                    let (t10, t02) = residual_periods(p);
                    let (m0, m_1, m2) = fitted_moments(&fit);
                    let mut errors = vec![(m_1 / m0 / t10).ln()];
                    if let Some(t02) = t02 {
                        errors.push(((m0 / m2).sqrt() / t02).ln());
                    }
                    (errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64).sqrt()
                });
                let error = |top_frequency: f64| {
                    let (mut diff, mut total) = (0.0, 0.0);
                    for k in short.iter().copied().filter(|k| f[*k] <= top_frequency) {
                        let est = fit.map_or(0.0, |fit| fit.density(f[k]));
                        diff += (est - s[k]).abs() * df[k];
                        total += s[k] * df[k];
                    }
                    diff / total
                };
                Some(Score {
                    fit: spectrum.fit,
                    peak_error: fit.map(|fit| (fit.peak_period - true_peak).abs()),
                    fitted_peak: fit.map(|fit| fit.peak_period),
                    true_peak,
                    shape_error: error(f64::INFINITY),
                    shape_error_axis: error(0.491),
                    gamma: fit.map(|fit| fit.gamma),
                    moment_misfit,
                })
            };
            let (Some(ifs_score), Some(aifs_score)) = (score(&ifs), score(&aifs)) else {
                println!("{}: no fit", record.date);
                continue;
            };

            // Energy on the axis (as a chart would integrate it).
            let spectrum = fitter.fit(&ifs).unwrap();
            let axis_m0: f64 = spectrum
                .density
                .iter()
                .zip(ecmwf_spectrum_bandwidths())
                .map(|(d, w)| d * w)
                .sum();
            // Energy of the bands plus the whole fitted part.
            let full_m0 = band_heights.iter().map(|h| h * h / 16.0).sum::<f64>()
                + spectrum.short_period.map_or(0.0, |fit| fit.energy);
            hs_check.push(4.0 * full_m0.sqrt() / hs);

            cases.push(Case {
                label: record.date.format("%Y-%m-%d %Hz").to_string(),
                tp,
                hs,
                short_peaks,
                ifs: ifs_score,
                aifs: aifs_score,
                axis_hs_ratio: 4.0 * axis_m0.sqrt() / hs,
            });
        }
    }

    println!(
        "{} GFS spectra from {}, fitted shape up to {:.3} Hz\n",
        cases.len(),
        paths.join(", "),
        fitter.map_or(0.0, |(t, _)| t)
    );

    let mut hs_sorted = hs_check.clone();
    hs_sorted.sort_by(f64::total_cmp);
    println!(
        "Hs of bands + fitted part / published Hs: min {:.4} max {:.4}",
        hs_sorted[0],
        hs_sorted[hs_sorted.len() - 1]
    );
    let axis: Vec<f64> = cases.iter().map(|c| c.axis_hs_ratio).collect();
    println!(
        "Hs integrated over the axis (stops at 0.49 Hz) / published Hs: median {:.3}, p10 {:.3}, min {:.3}\n",
        percentile(&axis, 50.0),
        percentile(&axis, 10.0),
        percentile(&axis, 0.0)
    );

    let groups: [(&str, Box<dyn Fn(&Case) -> bool>); 7] = [
        ("all", Box::new(|_| true)),
        ("Tp < 10 s", Box::new(|c| c.tp < 10.0)),
        ("Tp >= 10 s", Box::new(|c| c.tp >= 10.0)),
        (
            "  with a peak below 10 s",
            Box::new(|c| c.tp >= 10.0 && !c.short_peaks.is_empty()),
        ),
        (
            "  swell flank only",
            Box::new(|c| c.tp >= 10.0 && c.short_peaks.is_empty()),
        ),
        (
            "one peak below 10 s",
            Box::new(|c| c.short_peaks.len() <= 1),
        ),
        (
            "8-10 s peak + 2nd peak < 10 s",
            Box::new(|c| {
                c.short_peaks.len() >= 2 && c.short_peaks.iter().any(|p| (8.0..10.0).contains(p))
            }),
        ),
    ];
    for (name, scores) in [
        (
            "IFS (Hs, Tp, Tm-1,0, Tm02, bands)",
            (|c: &Case| &c.ifs) as fn(&Case) -> &Score,
        ),
        ("AIFS (Hs, Tm-1,0, bands; gamma 3.3)", |c: &Case| &c.aifs),
    ] {
        println!("{name}");
        println!(
            "  {:<30} {:>4}  {:>18}  {:>9}  {:>11}  {:>16}",
            "", "n", "peak err med / p90", "within 1s", "shape med", "shape to 0.49Hz"
        );
        for (label, filter) in &groups {
            let selected: Vec<&Score> = cases.iter().filter(|c| filter(c)).map(scores).collect();
            if selected.is_empty() {
                println!("  {label:<30} {:>4}", 0);
                continue;
            }
            let peak: Vec<f64> = selected.iter().filter_map(|s| s.peak_error).collect();
            let shape: Vec<f64> = selected.iter().map(|s| s.shape_error).collect();
            let shape_axis: Vec<f64> = selected.iter().map(|s| s.shape_error_axis).collect();
            println!(
                "  {:<30} {:>4}  {:>7.2} s / {:>4.2} s  {:>8.0}%  {:>11.2}  {:>16.2}",
                label,
                selected.len(),
                percentile(&peak, 50.0),
                percentile(&peak, 90.0),
                100.0 * peak.iter().filter(|e| **e <= 1.0).count() as f64 / peak.len() as f64,
                percentile(&shape, 50.0),
                percentile(&shape_axis, 50.0),
            );
        }
        let anchored = cases
            .iter()
            .filter(|c| scores(c).fit == FitQuality::PeakAnchored)
            .count();
        let mut gammas: Vec<(f64, usize)> = Vec::new();
        for g in cases.iter().filter_map(|c| scores(c).gamma) {
            match gammas.iter_mut().find(|(v, _)| *v == g) {
                Some((_, n)) => *n += 1,
                None => gammas.push((g, 1)),
            }
        }
        gammas.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!(
            "  PeakAnchored: {anchored}/{}; gamma counts {:?}",
            cases.len(),
            gammas
        );
        for (label, at_floor) in [("gamma 1 (grid floor)", true), ("gamma > 1", false)] {
            let misfit: Vec<f64> = cases
                .iter()
                .map(scores)
                .filter(|s| s.gamma.map(|g| (g == 1.0) == at_floor).unwrap_or(false))
                .filter_map(|s| s.moment_misfit)
                .collect();
            println!(
                "  {label}: n {}, period misfit (rms log) median {:.3} p90 {:.3}",
                misfit.len(),
                percentile(&misfit, 50.0),
                percentile(&misfit, 90.0)
            );
        }
        println!();
    }

    println!("Worst IFS peak errors:");
    let mut worst: Vec<&Case> = cases.iter().collect();
    worst.sort_by(|a, b| {
        b.ifs
            .peak_error
            .unwrap_or(0.0)
            .total_cmp(&a.ifs.peak_error.unwrap_or(0.0))
    });
    for c in worst.iter().take(12) {
        println!(
            "  {}  Hs {:.2} Tp {:>5.2}  true peak <10s {:>5.2}  fitted {:>5.2}  shape {:.2}  peaks <10s {:?}",
            c.label,
            c.hs,
            c.tp,
            c.ifs.true_peak,
            c.ifs.fitted_peak.unwrap_or(f64::NAN),
            c.ifs.shape_error,
            c.short_peaks.iter().map(|p| (p * 100.0).round() / 100.0).collect::<Vec<_>>()
        );
    }
}

/// Tm-1,0 and Tm02 of the energy outside the bands, as the fitter computes them.
fn residual_periods(p: &ECMWFWaveParameters) -> (f64, Option<f64>) {
    let mut m0 = p.significant_wave_height.powi(2) / 16.0;
    let mut m_1 = p.energy_period * m0;
    let mut m2 = p.zero_crossing_period.map(|t| m0 / (t * t));
    for ((min, max), h) in ECMWF_WAVE_PERIOD_BANDS.iter().zip(p.band_heights) {
        let (f1, f2) = (1.0 / *max as f64, 1.0 / *min as f64);
        let d = h * h / 16.0 / (f2 - f1);
        m0 -= d * (f2 - f1);
        m_1 -= d * (f2 / f1).ln();
        if let Some(m2) = m2.as_mut() {
            *m2 -= d * (f2.powi(3) - f1.powi(3)) / 3.0;
        }
    }
    (m_1 / m0, m2.map(|m2| (m0 / m2).sqrt()))
}

/// m0, m-1, m2 of a fitted shape over its range, by the midpoint rule in ln f.
fn fitted_moments(fit: &JonswapFit) -> (f64, f64, f64) {
    let n = 4000;
    let (a, b) = (0.1f64.ln(), fit.max_frequency.ln());
    let h = (b - a) / n as f64;
    let (mut m0, mut m_1, mut m2) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let f = (a + h * (i as f64 + 0.5)).exp();
        let e = fit.density(f) * f * h;
        m0 += e;
        m_1 += e / f;
        m2 += e * f * f;
    }
    (m0, m_1, m2)
}

/// WW3 bin edges: geometric midpoints between frequencies.
fn geometric_edges(f: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = f.len();
    let mid: Vec<f64> = f.windows(2).map(|w| (w[0] * w[1]).sqrt()).collect();
    let lo = std::iter::once(f[0] * f[0] / mid[0])
        .chain(mid.iter().copied())
        .collect();
    let hi = mid
        .iter()
        .copied()
        .chain(std::iter::once(f[n - 1] * f[n - 1] / mid[n - 2]))
        .collect();
    (lo, hi)
}

fn argmax(values: &[f64]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap()
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let rank = p / 100.0 * (v.len() - 1) as f64;
    let (i, frac) = (rank.floor() as usize, rank.fract());
    if i + 1 < v.len() {
        v[i] + frac * (v[i + 1] - v[i])
    } else {
        v[i]
    }
}
