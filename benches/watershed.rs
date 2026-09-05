//! Run with `cargo bench --bench watershed`. Fixture loading and buoy spectrum
//! reconstruction happen outside the timed region; blur and result disposal are
//! included. Report medians of batch averages, not individual-call percentiles.
#[path = "../tests/support/watershed.rs"]
mod support;

use std::{hint::black_box, time::Instant};

fn main() {
    for (name, spectra, blur) in [
        ("forecast", support::forecast_spectra(), None),
        ("buoy", support::buoy_spectra(), Some(0.8)),
    ] {
        let samples = (0..32)
            .map(|i| &spectra[i * spectra.len() / 32])
            .collect::<Vec<_>>();
        let mut times = Vec::new();
        for round in 0..9 {
            let start = Instant::now();
            for _ in 0..16 {
                for spectrum in &samples {
                    black_box(black_box(spectrum).partition(100, blur).unwrap());
                }
            }
            let micros = start.elapsed().as_secs_f64() * 1e6 / (16 * samples.len()) as f64;
            if round > 0 {
                times.push(micros);
            }
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{name} {}x{}, blur {blur:?}: {:.2} us/call (batch range {:.2}-{:.2})",
            samples[0].nk(),
            samples[0].nth(),
            (times[3] + times[4]) / 2.0,
            times[0],
            times[7],
        );
    }
}
