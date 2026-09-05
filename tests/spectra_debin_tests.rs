use std::fs;

use surfrs::data::swan_spectral_file::SwanSpectralFile;
use surfrs::spectra::Spectra;

fn read_mock_spectra(name: &str) -> Spectra {
    let data = fs::read_to_string(format!("mock/{}", name)).unwrap();
    let file: SwanSpectralFile = data.parse().unwrap();
    file.frames[0].spectra[0].as_ref().unwrap().clone()
}

/// Circular bin widths in degrees for a sorted direction axis in degrees
fn circular_bin_widths_deg(direction: &[f64]) -> Vec<f64> {
    let count = direction.len();
    (0..count)
        .map(|ith| {
            let up = (direction[(ith + 1) % count] - direction[ith]).rem_euclid(360.0);
            let down = (direction[ith] - direction[(ith + count - 1) % count]).rem_euclid(360.0);
            (up + down) / 2.0
        })
        .collect()
}

/// (m0, mean direction in degrees, r1) for one frequency row
fn row_moments(spectra: &Spectra, ik: usize) -> (f64, f64, f64) {
    let direction = spectra.direction_deg();
    let widths = circular_bin_widths_deg(&direction);
    let mut m0 = 0.0;
    let mut esin = 0.0;
    let mut ecos = 0.0;
    for ith in 0..spectra.nth() {
        let energy = spectra.energy_at(ik, ith);
        if !energy.is_finite() {
            continue;
        }
        let binned = energy * widths[ith].to_radians();
        let rad = direction[ith].to_radians();
        m0 += binned;
        ecos += binned * rad.cos();
        esin += binned * rad.sin();
    }
    if m0 <= 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        let a1 = ecos / m0;
        let b1 = esin / m0;
        (
            m0,
            b1.atan2(a1).to_degrees().rem_euclid(360.0),
            a1.hypot(b1),
        )
    }
}

fn spread_deg(r1: f64) -> f64 {
    (2.0 * (1.0 - r1)).max(0.0).sqrt().to_degrees()
}

#[test]
fn debin_matches_gate0_reference_output() {
    // Hurricane Lee boundary spectrum from the hopewaves Gate 0 experiment:
    // 36 x 10 degree bins de-binned by the validated Python reference
    let binned = read_mock_spectra("gate0.binned.bnd");
    let expected = read_mock_spectra("gate0.debinned.bnd");

    let debinned = binned.debin();

    assert_eq!(debinned.nk(), expected.nk());
    assert_eq!(debinned.nth(), expected.nth());
    for (actual, reference) in debinned
        .direction_deg()
        .iter()
        .zip(expected.direction_deg())
    {
        assert!((actual - reference).abs() < 1.0e-9);
    }

    for ik in 0..debinned.nk() {
        for ith in 0..debinned.nth() {
            let actual = debinned.energy_at(ik, ith);
            let reference = expected.energy_at(ik, ith);
            let magnitude = actual.abs().max(reference.abs());
            // The reference file carries 10 significant digits; densities
            // this far below the spectral peak are physically zero
            if magnitude < 1.0e-100 {
                continue;
            }
            assert!(
                (actual - reference).abs() <= 1.0e-9 * magnitude,
                "density mismatch at ik={ik} ith={ith}: {actual} vs {reference}"
            );
        }
    }
}

#[test]
fn debin_round_trips_gate0_row_moments() {
    let binned = read_mock_spectra("gate0.binned.bnd");
    let debinned = binned.debin();

    for ik in 0..binned.nk() {
        let (m0_in, dir_in, r1_in) = row_moments(&binned, ik);
        let (m0_out, dir_out, r1_out) = row_moments(&debinned, ik);
        if m0_in <= 0.0 {
            assert_eq!(m0_out, 0.0);
            continue;
        }
        assert!((m0_out / m0_in - 1.0).abs() < 1.0e-12);
        let dir_error = (dir_out - dir_in + 180.0).rem_euclid(360.0) - 180.0;
        assert!(dir_error.abs() < 0.01);
        assert!((spread_deg(r1_out) - spread_deg(r1_in)).abs() < 0.01);
    }
}
