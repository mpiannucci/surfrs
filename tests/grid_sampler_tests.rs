use std::fs;

use gribberish::message::{read_messages, Message};
use surfrs::{
    location::Location,
    tools::grid_sampler::{GridSampler, SampleMethod, SampleSource},
};

// All fixtures are the 2026-09-23 00z run, 24 hour forecast.
const GFS_GLOBAL_0P16: &str = "mock/gfswave.20260923.t00z.global.0p16.f024.htsgw.grib2";
const GFS_ATLOCN_0P16: &str = "mock/gfswave.20260923.t00z.atlocn.0p16.f024.htsgw.grib2";
const GFS_GLOBAL_0P25: &str = "mock/gfswave.20260923.t00z.global.0p25.f024.htsgw.grib2";
const ECMWF_IFS_0P25: &str = "mock/ecmwf/ifs.20260923000000-24h-wave-fc.grib2";

// Hst for 2026-09-24 00z from gfswave.t00z.bull_tar, gfswave.44097.bull. The
// bulletin is computed by WAVEWATCH III at the station, not from the grids.
const BULLETIN_44097_HST: f64 = 2.89;

/// Sampler for the significant wave height message in a fixture.
fn sampler(path: &str) -> GridSampler {
    let data = fs::read(path).expect("fixture not found");
    let message = read_messages(&data)
        .find(is_significant_height)
        .expect("fixture has no significant wave height");
    GridSampler::from_message(&message).unwrap()
}

fn is_significant_height(message: &Message) -> bool {
    message.variable_abbrev().ok().as_deref() == Some("HTSGW")
        && message.wave_period_range().ok().flatten().is_none()
}

fn buoy_44097() -> Location {
    Location::new(40.97, -71.13, "NDBC 44097".into())
}

#[test]
fn energy_interpolation_on_native_grid_matches_station_output() {
    // global.0p16 is the native gnh_10m computational grid, so energy weighted
    // bilinear reproduces the WW3 station output up to 0.01 m packing.
    let sample = sampler(GFS_GLOBAL_0P16).sample(&buoy_44097(), SampleMethod::BilinearEnergy);
    let value = sample.value.unwrap();
    assert!(
        (value - BULLETIN_44097_HST).abs() < 0.02,
        "{value} vs bulletin {BULLETIN_44097_HST}"
    );
    assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 4 });
}

#[test]
fn energy_interpolation_on_regridded_grid_is_close_to_station_output() {
    // global.0p25 is interpolated from the native grid, so it is slightly further off.
    let value = sampler(GFS_GLOBAL_0P25)
        .sample(&buoy_44097(), SampleMethod::BilinearEnergy)
        .value
        .unwrap();
    assert!(
        (value - BULLETIN_44097_HST).abs() < 0.05,
        "{value} vs bulletin {BULLETIN_44097_HST}"
    );
}

#[test]
fn grids_starting_at_different_longitudes_agree() {
    // global.0p16 starts at 0 degrees, atlocn.0p16 at 260 degrees. Near 44097
    // both hold the same cells.
    let global = sampler(GFS_GLOBAL_0P16)
        .sample(&buoy_44097(), SampleMethod::BilinearEnergy)
        .value
        .unwrap();
    let regional = sampler(GFS_ATLOCN_0P16)
        .sample(&buoy_44097(), SampleMethod::BilinearEnergy)
        .value
        .unwrap();
    assert!((global - regional).abs() < 1e-3, "{global} vs {regional}");
}

#[test]
fn ecmwf_grid_starting_at_antimeridian_matches_eccodes() {
    // ECMWF grids start at 180 degrees. eccodes picks 41.00N 71.25W as the nearest
    // point to 44097, with value 3.56325.
    let sample = sampler(ECMWF_IFS_0P25).sample(&buoy_44097(), SampleMethod::Nearest);
    assert!((sample.value.unwrap() - 3.56325).abs() < 1e-4);
    assert_eq!(sample.source, SampleSource::NearestCell);
}

#[test]
fn masked_corners_are_dropped() {
    // Long Island: 40.75N 73.00W is land, the other three corners are water.
    let location = Location::new(40.80, -72.80, "Long Island".into());
    let sample = sampler(ECMWF_IFS_0P25).sample(&location, SampleMethod::BilinearEnergy);
    assert!(sample.value.is_some());
    assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 3 });
}

#[test]
fn land_point_falls_back_to_nearest_sea_cell() {
    // Inland New Jersey: all four corners are land. The closest water cell is
    // 40.00N 74.00W (3.63112 m per eccodes), about 28 km away.
    let location = Location::new(40.1, -74.3, "Jackson, NJ".into());
    let sample = sampler(ECMWF_IFS_0P25).sample(&location, SampleMethod::BilinearEnergy);
    assert!((sample.value.unwrap() - 3.63112).abs() < 1e-4);
    match sample.source {
        SampleSource::NearestSeaCell { distance_km } => {
            assert!(distance_km > 25.0 && distance_km < 30.0, "{distance_km}")
        }
        other => panic!("unexpected source {other:?}"),
    }

    // Far inland there is nothing within the fallback distance.
    let kansas = Location::new(38.5, -98.0, "Kansas".into());
    let sample = sampler(ECMWF_IFS_0P25).sample(&kansas, SampleMethod::BilinearEnergy);
    assert_eq!(sample.value, None);
    assert_eq!(sample.source, SampleSource::Missing);
}

mod projected {
    use super::*;

    // HRRR 2026-09-23 00z analysis, 2 m temperature (K). Lambert conformal,
    // 1799 x 1059 at 3 km, LoV 262.5, standard parallel 38.5.
    const HRRR_TMP_2M: &str = "mock/hrrr.20260923.t00z.wrfsfcf00.tmp2m.grib2";

    fn hrrr() -> GridSampler {
        let data = fs::read(HRRR_TMP_2M).unwrap();
        let message = read_messages(&data).next().unwrap();
        GridSampler::from_message(&message).unwrap()
    }

    #[test]
    fn nearest_matches_eccodes_across_conus() {
        // (lat, lon, eccodes nearest value from `grib_get -l lat,lon,1`)
        let cases = [
            (40.97, -71.13, 290.548),
            (34.0, -118.25, 297.048),
            (47.6, -122.3, 292.861),
            (25.8, -80.2, 299.861),
            (39.74, -104.99, 300.173),
        ];
        let grid = hrrr();
        for (lat, lon, expected) in cases {
            let sample = grid.sample(&Location::new(lat, lon, "".into()), SampleMethod::Nearest);
            let value = sample.value.unwrap();
            assert!(
                (value - expected).abs() < 1e-3,
                "{lat},{lon}: {value} vs {expected}"
            );
            assert_eq!(sample.source, SampleSource::NearestCell);

            // Positive longitudes index the same cell.
            let positive = grid.sample(
                &Location::new(lat, lon + 360.0, "".into()),
                SampleMethod::Nearest,
            );
            assert_eq!(positive, sample);
        }
    }

    #[test]
    fn bilinear_stays_between_neighbours() {
        // eccodes' four nearest cells to 44097: 290.548, 290.673, 290.486, 290.548
        let sample = hrrr().sample(&buoy_44097(), SampleMethod::Bilinear);
        let value = sample.value.unwrap();
        assert!(
            value >= 290.486 - 1e-3 && value <= 290.673 + 1e-3,
            "{value}"
        );
        assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 4 });
    }

    #[test]
    fn points_off_the_grid_are_missing() {
        let grid = hrrr();
        for (lat, lon) in [(21.3, -157.9), (51.5, -0.13), (-33.9, 151.2)] {
            let sample = grid.sample(&Location::new(lat, lon, "".into()), SampleMethod::Bilinear);
            assert_eq!(sample.source, SampleSource::Missing, "{lat},{lon}");
        }
    }
}
