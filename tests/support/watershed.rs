use std::{f64::consts::PI, fs, path::Path};

use surfrs::{
    data::{
        directional_spectral_wave_data_record::DirectionalSpectralWaveDataRecord,
        forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecordCollection,
        spectral_wave_data_record::SpectralWaveDataRecordCollection,
    },
    spectra::Spectra,
};

fn read_mock(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("mock")
            .join(name),
    )
    .unwrap()
}

pub fn forecast_spectra() -> Vec<Spectra> {
    let raw = read_mock("gfswave.44097.spec");
    let mut collection = ForecastSpectralWaveDataRecordCollection::from_data(&raw);
    let spectra = collection.records().unwrap().1.map(|r| r.spectra).collect();
    spectra
}

pub fn buoy_spectra() -> Vec<Spectra> {
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
