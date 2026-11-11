//! Scores the ECMWF fitted spectrum against GFS-Wave point spectra.
//!
//! For each GFS spectrum: compute what ECMWF would publish (Hs, Tp, Tm-1,0,
//! Tm02, six band heights), rebuild the curve, and compare the part below 10 s
//! with the truth. GFS spectra reach 0.964 Hz, close to ECWAM's 0.97 Hz.
//!
//! ```sh
//! cargo run --release --example validate_ecmwf_spectrum_fit [gfswave.spec ...]
//! ```
//!
//! Defaults to `mock/gfswave.44097.spec`.

use surfrs::{
    data::{
        ecmwf_field::ECMWF_WAVE_PERIOD_BANDS, ecmwf_fitted_spectrum::ECMWFWaveParameters,
        forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecordCollection,
    },
    spectra::SpectralAxis,
};

struct Case {
    tp: f64,
    /// Local maxima below 10 s above 25% of the largest bin there, s.
    short_peaks: Vec<f64>,
    /// (peak error s, shape error) for IFS and AIFS inputs.
    ifs: (f64, f64),
    aifs: (f64, f64),
}

fn main() {
    let mut paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        paths.push("mock/gfswave.44097.spec".into());
    }

    let mut cases = Vec::new();
    for path in &paths {
        let raw = std::fs::read_to_string(path).expect("spec file");
        let mut collection = ForecastSpectralWaveDataRecordCollection::from_data(&raw);
        for record in collection.records().expect("spec records").1 {
            let f = record.spectra.frequency.clone();
            let s = record.spectra.oned(SpectralAxis::Frequency);
            // WW3 bin edges at the geometric midpoints.
            let r = (f[1] / f[0]).sqrt();
            let (lo, hi): (Vec<f64>, Vec<f64>) = f.iter().map(|f| (f / r, f * r)).unzip();
            let df: Vec<f64> = lo.iter().zip(&hi).map(|(a, b)| b - a).collect();
            let moment =
                |n: i32| -> f64 { (0..f.len()).map(|k| s[k] * f[k].powi(n) * df[k]).sum() };
            let (m0, m_1, m2) = (moment(0), moment(-1), moment(2));

            let short: Vec<usize> = (0..f.len()).filter(|k| f[*k] > 0.1).collect();
            let short_total: f64 = short.iter().map(|k| s[*k] * df[*k]).sum();
            if m0 <= 0.0 || short_total <= 0.0 {
                continue;
            }
            let ifs = ECMWFWaveParameters {
                significant_wave_height: 4.0 * m0.sqrt(),
                energy_period: m_1 / m0,
                zero_crossing_period: Some((m0 / m2).sqrt()),
                peak_period: Some(1.0 / f[argmax(&s, 0..f.len())]),
                band_heights: ECMWF_WAVE_PERIOD_BANDS.map(|(min, max)| {
                    let (f1, f2) = (1.0 / max as f64, 1.0 / min as f64);
                    let e: f64 = (0..f.len())
                        .map(|k| s[k] * (hi[k].min(f2) - lo[k].max(f1)).max(0.0))
                        .sum();
                    4.0 * e.sqrt()
                }),
            };
            let aifs = ECMWFWaveParameters {
                zero_crossing_period: None,
                peak_period: None,
                ..ifs.clone()
            };

            let true_peak = 1.0 / f[argmax(&s, short.clone().into_iter())];
            let largest = short.iter().map(|k| s[*k]).fold(0.0, f64::max);
            let short_peaks = short
                .windows(3)
                .filter(|w| s[w[1]] >= s[w[0]] && s[w[1]] > s[w[2]] && s[w[1]] > 0.25 * largest)
                .map(|w| 1.0 / f[w[1]])
                .collect();
            let score = |p: &ECMWFWaveParameters| {
                let spectrum = p.fitted_spectrum(*hi.last().unwrap()).expect("fit");
                let peak = spectrum
                    .short_period
                    .map_or(f64::NAN, |fit| fit.peak_period);
                let shape: f64 = short
                    .iter()
                    .map(|k| (spectrum.density(f[*k]) - s[*k]).abs() * df[*k])
                    .sum();
                ((peak - true_peak).abs(), shape / short_total)
            };

            cases.push(Case {
                tp: ifs.peak_period.unwrap(),
                short_peaks,
                ifs: score(&ifs),
                aifs: score(&aifs),
            });
        }
    }

    println!("{} GFS spectra from {}\n", cases.len(), paths.join(", "));
    let groups: [(&str, fn(&Case) -> bool); 7] = [
        ("all", |_| true),
        ("Tp < 10 s", |c| c.tp < 10.0),
        ("Tp >= 10 s", |c| c.tp >= 10.0),
        ("  with a peak below 10 s", |c| {
            c.tp >= 10.0 && !c.short_peaks.is_empty()
        }),
        ("  swell flank only", |c| {
            c.tp >= 10.0 && c.short_peaks.is_empty()
        }),
        ("one peak below 10 s", |c| c.short_peaks.len() <= 1),
        ("8-10 s peak + 2nd below 10 s", |c| {
            c.short_peaks.len() >= 2 && c.short_peaks.iter().any(|p| (8.0..10.0).contains(p))
        }),
    ];
    for (name, score) in [
        ("IFS", (|c: &Case| c.ifs) as fn(&Case) -> (f64, f64)),
        ("AIFS (no Tp, no Tm02)", |c: &Case| c.aifs),
    ] {
        println!("{name:<30} {:>4}  peak err med / p90  shape med", "n");
        for (label, keep) in &groups {
            let scores: Vec<(f64, f64)> = cases.iter().filter(|c| keep(c)).map(score).collect();
            let peak: Vec<f64> = scores
                .iter()
                .map(|s| s.0)
                .filter(|e| e.is_finite())
                .collect();
            let shape: Vec<f64> = scores.iter().map(|s| s.1).collect();
            println!(
                "  {label:<28} {:>4}  {:>6.2} / {:>5.2} s  {:>9.2}",
                scores.len(),
                percentile(&peak, 50.0),
                percentile(&peak, 90.0),
                percentile(&shape, 50.0)
            );
        }
        println!();
    }
}

fn argmax(s: &[f64], indices: impl Iterator<Item = usize>) -> usize {
    indices.max_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap()
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let rank = p / 100.0 * (v.len() - 1) as f64;
    let i = rank.floor() as usize;
    v[i] + rank.fract() * (v[(i + 1).min(v.len() - 1)] - v[i])
}
