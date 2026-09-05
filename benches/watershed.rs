//! Run with `cargo bench --bench watershed`. Fixture loading and buoy spectrum
//! reconstruction happen outside the timed region; blur and result disposal are
//! included. Report medians of batch averages, not individual-call percentiles.
use std::{f64::consts::PI, fs, hint::black_box, path::Path, time::Instant};

use surfrs::{
    data::{
        directional_spectral_wave_data_record::DirectionalSpectralWaveDataRecord,
        forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecordCollection,
        spectral_wave_data_record::SpectralWaveDataRecordCollection,
    },
    spectra::Spectra,
};

fn main() {
    for (name, spectra, blur) in [
        ("forecast", forecast_spectra(), None),
        ("buoy", buoy_spectra(), Some(0.8)),
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

fn read_mock(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mock")
            .join(name),
    )
    .unwrap()
}

fn forecast_spectra() -> Vec<Spectra> {
    let raw = read_mock("gfswave.44097.spec");
    let mut collection = ForecastSpectralWaveDataRecordCollection::from_data(&raw);
    let spectra = collection.records().unwrap().1.map(|r| r.spectra).collect();
    spectra
}

fn buoy_spectra() -> Vec<Spectra> {
    let raw = ["data_spec", "swdir", "swdir2", "swr1", "swr2"]
        .map(|extension| read_mock(&format!("44097.{extension}")));
    let [energy, mean_direction, primary_direction, first, second] = &raw;
    let mut energy = SpectralWaveDataRecordCollection::from_data(energy);
    let mut mean_direction = SpectralWaveDataRecordCollection::from_data(mean_direction);
    let mut primary_direction = SpectralWaveDataRecordCollection::from_data(primary_direction);
    let mut first = SpectralWaveDataRecordCollection::from_data(first);
    let mut second = SpectralWaveDataRecordCollection::from_data(second);
    let direction = (0..36)
        .map(|i| (2.0 * PI / 36.0) * i as f64)
        .collect::<Vec<_>>();

    itertools::izip!(
        energy.records(),
        mean_direction.records(),
        primary_direction.records(),
        first.records(),
        second.records(),
    )
    .map(|(e, mwd, pwd, r1, r2)| {
        DirectionalSpectralWaveDataRecord::from_data_records(&direction, e, mwd, pwd, r1, r2)
            .spectra
    })
    .collect()
}
