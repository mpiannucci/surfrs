use gribberish::{error::GribberishError, message::Message};
use serde::{Deserialize, Serialize};

use crate::location::Location;

const EARTH_RADIUS_KM: f64 = 6371.0;
const KM_PER_DEGREE: f64 = 111.195;
const DEFAULT_MAX_FALLBACK_KM: f64 = 30.0;

/// How to turn the grid cells around a point into a value at the point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleMethod {
    /// The value of the closest grid cell.
    Nearest,
    /// Bilinear interpolation of the value. Use for periods and other scalars.
    Bilinear,
    /// Bilinear interpolation of the squared value, then the square root. Use for
    /// wave heights: it interpolates energy, which is how WAVEWATCH III builds its
    /// station output (w3iopomd.F90, W3IOPE).
    BilinearEnergy,
    /// Bilinear interpolation of unit vectors. Use for directions in degrees.
    BilinearAngular,
}

/// Where a sampled value came from.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SampleSource {
    /// Interpolated from `sea_cells` (1-4) valid corners, with the weights of
    /// masked (land) corners dropped and the rest renormalised.
    Interpolated { sea_cells: u8 },
    /// The closest grid cell was valid.
    NearestCell,
    /// Every candidate cell was masked, so the closest valid cell within the
    /// fallback distance was used.
    NearestSeaCell { distance_km: f64 },
    /// The point is off the grid or no valid cell is within the fallback distance.
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointSample {
    pub value: Option<f64>,
    pub source: SampleSource,
}

impl PointSample {
    fn missing() -> Self {
        PointSample {
            value: None,
            source: SampleSource::Missing,
        }
    }
}

/// A decoded regular latitude/longitude grid that can be sampled at points.
///
/// Decode a message once with `from_message` and sample as many locations as
/// needed from it. Masked cells (land) are expected to be NaN.
#[derive(Clone, Debug)]
pub struct GridSampler {
    lat_start: f64,
    lat_step: f64,
    lat_count: usize,
    lng_start: f64,
    lng_step: f64,
    lng_count: usize,
    is_global: bool,
    max_fallback_km: f64,
    data: Vec<f64>,
}

impl GridSampler {
    pub fn from_message(message: &Message) -> Result<Self, GribberishError> {
        let projector = message.latlng_projector()?;
        if !projector.is_regular_latlng_grid() {
            return Err(GribberishError::MessageError(
                "point sampling requires a regular lat/lng grid".into(),
            ));
        }

        let (lat_count, lng_count) = message.grid_dimensions()?;
        if lat_count < 2 || lng_count < 2 {
            return Err(GribberishError::MessageError(
                "point sampling requires at least a 2x2 grid".into(),
            ));
        }

        let (lat_start, lng_start) = projector.latlng_start();
        let (lat_end, lng_end) = projector.latlng_end();

        Self::from_regular_grid(
            lat_start,
            (lat_end - lat_start) / (lat_count - 1) as f64,
            lat_count,
            lng_start,
            (lng_end - lng_start) / (lng_count - 1) as f64,
            lng_count,
            message.data()?,
        )
    }

    /// Build a sampler from grid geometry and row-major data (latitude rows,
    /// longitude fastest).
    pub fn from_regular_grid(
        lat_start: f64,
        lat_step: f64,
        lat_count: usize,
        lng_start: f64,
        lng_step: f64,
        lng_count: usize,
        data: Vec<f64>,
    ) -> Result<Self, GribberishError> {
        if data.len() != lat_count * lng_count {
            return Err(GribberishError::MessageError(format!(
                "grid data has {} values, expected {lat_count}x{lng_count}",
                data.len()
            )));
        }
        if lat_step == 0.0 || lng_step == 0.0 {
            return Err(GribberishError::MessageError(
                "grid steps must be non-zero".into(),
            ));
        }

        let is_global = (lng_count as f64 * lng_step.abs() - 360.0).abs() < lng_step.abs() / 2.0;

        Ok(GridSampler {
            lat_start,
            lat_step,
            lat_count,
            lng_start,
            lng_step,
            lng_count,
            is_global,
            max_fallback_km: DEFAULT_MAX_FALLBACK_KM,
            data,
        })
    }

    /// The furthest a masked point may fall back to the nearest valid cell.
    pub fn with_max_fallback_km(mut self, km: f64) -> Self {
        self.max_fallback_km = km;
        self
    }

    pub fn sample(&self, location: &Location, method: SampleMethod) -> PointSample {
        let Some((x, y)) = self.fractional_index(location.latitude, location.longitude) else {
            return PointSample::missing();
        };

        let sample = match method {
            SampleMethod::Nearest => self.nearest(x, y),
            _ => self.bilinear(x, y, method),
        };

        match sample.value {
            Some(_) => sample,
            None => self.nearest_sea_cell(location, x, y),
        }
    }

    pub fn sample_many(&self, locations: &[Location], method: SampleMethod) -> Vec<PointSample> {
        locations.iter().map(|l| self.sample(l, method)).collect()
    }

    /// Fractional (column, row) of a point, or None when it is off the grid.
    fn fractional_index(&self, latitude: f64, longitude: f64) -> Option<(f64, f64)> {
        const EPSILON: f64 = 1e-6;

        let y = (latitude - self.lat_start) / self.lat_step;
        if y < -EPSILON || y > (self.lat_count - 1) as f64 + EPSILON {
            return None;
        }

        // Wrap into one revolution of columns so grids starting at 0, 180 or a
        // regional offset all index the same way.
        let columns_per_revolution = 360.0 / self.lng_step.abs();
        let x = ((longitude - self.lng_start) / self.lng_step).rem_euclid(columns_per_revolution);
        let x = if x > columns_per_revolution - EPSILON {
            0.0
        } else {
            x
        };
        if !self.is_global && x > (self.lng_count - 1) as f64 + EPSILON {
            return None;
        }

        Some((x, y.clamp(0.0, (self.lat_count - 1) as f64)))
    }

    fn value(&self, column: i64, row: i64) -> Option<f64> {
        if row < 0 || row >= self.lat_count as i64 {
            return None;
        }
        let column = if self.is_global {
            column.rem_euclid(self.lng_count as i64)
        } else if column < 0 || column >= self.lng_count as i64 {
            return None;
        } else {
            column
        };

        let value = self.data[row as usize * self.lng_count + column as usize];
        (!value.is_nan()).then_some(value)
    }

    fn nearest(&self, x: f64, y: f64) -> PointSample {
        match self.value(x.round() as i64, y.round() as i64) {
            Some(value) => PointSample {
                value: Some(value),
                source: SampleSource::NearestCell,
            },
            None => PointSample::missing(),
        }
    }

    fn bilinear(&self, x: f64, y: f64, method: SampleMethod) -> PointSample {
        let column = x.floor() as i64;
        let row = (y.floor() as i64).min(self.lat_count as i64 - 2);
        let tx = x - column as f64;
        let ty = y - row as f64;

        let corners = [
            (column, row, (1.0 - tx) * (1.0 - ty)),
            (column + 1, row, tx * (1.0 - ty)),
            (column, row + 1, (1.0 - tx) * ty),
            (column + 1, row + 1, tx * ty),
        ];

        let mut sea_cells = 0;
        let mut weight_sum = 0.0;
        let mut sum = 0.0;
        let mut sin_sum = 0.0;
        let mut cos_sum = 0.0;
        for (c, r, weight) in corners {
            let Some(value) = self.value(c, r) else {
                continue;
            };
            sea_cells += 1;
            weight_sum += weight;
            match method {
                SampleMethod::BilinearEnergy => sum += weight * value * value,
                SampleMethod::BilinearAngular => {
                    let radians = value.to_radians();
                    sin_sum += weight * radians.sin();
                    cos_sum += weight * radians.cos();
                }
                _ => sum += weight * value,
            }
        }

        if weight_sum <= 1e-9 {
            return PointSample::missing();
        }

        let value = match method {
            SampleMethod::BilinearEnergy => (sum / weight_sum).sqrt(),
            SampleMethod::BilinearAngular => sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0),
            _ => sum / weight_sum,
        };

        PointSample {
            value: Some(value),
            source: SampleSource::Interpolated { sea_cells },
        }
    }

    fn nearest_sea_cell(&self, location: &Location, x: f64, y: f64) -> PointSample {
        let row_reach =
            (self.max_fallback_km / (KM_PER_DEGREE * self.lat_step.abs())).ceil() as i64;
        // On a global grid, searching more than half way round would visit
        // columns twice.
        let max_column_reach = if self.is_global {
            self.lng_count as i64 / 2
        } else {
            self.lng_count as i64
        };
        let km_per_column =
            KM_PER_DEGREE * self.lng_step.abs() * location.latitude.to_radians().cos().abs();
        let column_reach = if km_per_column < 1e-6 {
            max_column_reach
        } else {
            ((self.max_fallback_km / km_per_column).ceil() as i64).min(max_column_reach)
        };

        let center_column = x.round() as i64;
        let center_row = y.round() as i64;

        let mut best: Option<(f64, f64)> = None;
        for row in (center_row - row_reach)..=(center_row + row_reach) {
            for column in (center_column - column_reach)..=(center_column + column_reach) {
                let Some(value) = self.value(column, row) else {
                    continue;
                };
                let distance = haversine_km(
                    location.latitude,
                    location.longitude,
                    self.lat_start + row as f64 * self.lat_step,
                    self.lng_start + column as f64 * self.lng_step,
                );
                if distance <= self.max_fallback_km && best.map_or(true, |(d, _)| distance < d) {
                    best = Some((distance, value));
                }
            }
        }

        match best {
            Some((distance_km, value)) => PointSample {
                value: Some(value),
                source: SampleSource::NearestSeaCell { distance_km },
            },
            None => PointSample::missing(),
        }
    }
}

fn haversine_km(lat_a: f64, lng_a: f64, lat_b: f64, lng_b: f64) -> f64 {
    let (phi_a, phi_b) = (lat_a.to_radians(), lat_b.to_radians());
    let d_phi = phi_b - phi_a;
    let d_lambda = (lng_b - lng_a).to_radians();
    let a =
        (d_phi / 2.0).sin().powi(2) + phi_a.cos() * phi_b.cos() * (d_lambda / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3x4 grid at 1 degree, starting at `lng_start`, value = 10 * row + column.
    fn small_grid(lng_start: f64) -> GridSampler {
        let data = (0..3)
            .flat_map(|r| (0..4).map(move |c| (10 * r + c) as f64))
            .collect();
        GridSampler::from_regular_grid(42.0, -1.0, 3, lng_start, 1.0, 4, data).unwrap()
    }

    fn at(lat: f64, lng: f64) -> Location {
        Location::new(lat, lng, "test".into())
    }

    #[test]
    fn test_bilinear_regional_grid() {
        let grid = small_grid(288.0);
        let sample = grid.sample(&at(41.5, -71.5), SampleMethod::Bilinear);
        // Rows 0-1, columns 0-1, halfway in both: (0 + 1 + 10 + 11) / 4
        assert!((sample.value.unwrap() - 5.5).abs() < 1e-9);
        assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 4 });

        // The same point written with a positive longitude.
        let positive = grid.sample(&at(41.5, 288.5), SampleMethod::Bilinear);
        assert_eq!(sample, positive);
    }

    #[test]
    fn test_outside_regional_grid_is_missing() {
        let grid = small_grid(288.0);
        assert_eq!(
            grid.sample(&at(41.0, -80.0), SampleMethod::Bilinear),
            PointSample::missing()
        );
        assert_eq!(
            grid.sample(&at(45.0, -71.0), SampleMethod::Bilinear),
            PointSample::missing()
        );
    }

    #[test]
    fn test_global_grid_start_longitude_does_not_matter() {
        // A global 1 degree grid whose value depends only on true longitude.
        let build = |lng_start: f64| {
            let data = (0..3)
                .flat_map(|_| {
                    (0..360).map(move |c| ((lng_start + c as f64).rem_euclid(360.0)).sin())
                })
                .collect();
            GridSampler::from_regular_grid(42.0, -1.0, 3, lng_start, 1.0, 360, data).unwrap()
        };

        let location = at(41.3, -71.13);
        let from_zero = build(0.0).sample(&location, SampleMethod::Bilinear);
        let from_antimeridian = build(180.0).sample(&location, SampleMethod::Bilinear);
        let from_negative = build(-180.0).sample(&location, SampleMethod::Bilinear);

        assert!((from_zero.value.unwrap() - from_antimeridian.value.unwrap()).abs() < 1e-12);
        assert!((from_zero.value.unwrap() - from_negative.value.unwrap()).abs() < 1e-12);
    }

    #[test]
    fn test_global_grid_wraps_between_last_and_first_column() {
        let data = (0..3).flat_map(|_| (0..360).map(|c| c as f64)).collect();
        let grid = GridSampler::from_regular_grid(42.0, -1.0, 3, 0.0, 1.0, 360, data).unwrap();

        // Halfway between column 359 (359) and column 0 (0).
        let sample = grid.sample(&at(41.0, 359.5), SampleMethod::Bilinear);
        assert!((sample.value.unwrap() - 179.5).abs() < 1e-9);
        assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 4 });
    }

    #[test]
    fn test_energy_interpolation() {
        let data = vec![1.0, 3.0, 1.0, 3.0];
        let grid = GridSampler::from_regular_grid(1.0, -1.0, 2, 0.0, 1.0, 2, data).unwrap();
        let location = at(0.5, 0.5);

        let linear = grid
            .sample(&location, SampleMethod::Bilinear)
            .value
            .unwrap();
        let energy = grid
            .sample(&location, SampleMethod::BilinearEnergy)
            .value
            .unwrap();
        assert!((linear - 2.0).abs() < 1e-9);
        assert!((energy - 5.0_f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn test_angular_interpolation_across_north() {
        let data = vec![350.0, 10.0, 350.0, 10.0];
        let grid = GridSampler::from_regular_grid(1.0, -1.0, 2, 0.0, 1.0, 2, data).unwrap();
        let direction = grid
            .sample(&at(0.5, 0.5), SampleMethod::BilinearAngular)
            .value
            .unwrap();
        assert!(direction < 1e-9 || (360.0 - direction) < 1e-9);
    }

    #[test]
    fn test_masked_corners_are_dropped() {
        let data = vec![2.0, f64::NAN, 4.0, f64::NAN];
        let grid = GridSampler::from_regular_grid(1.0, -1.0, 2, 0.0, 1.0, 2, data).unwrap();
        let sample = grid.sample(&at(0.5, 0.25), SampleMethod::Bilinear);
        assert!((sample.value.unwrap() - 3.0).abs() < 1e-9);
        assert_eq!(sample.source, SampleSource::Interpolated { sea_cells: 2 });
    }

    #[test]
    fn test_all_land_falls_back_to_nearest_sea_cell() {
        // 0.25 degree grid: only the far corner cell is water.
        let mut data = vec![f64::NAN; 16];
        data[15] = 1.5;
        let grid = GridSampler::from_regular_grid(41.0, -0.25, 4, 288.0, 0.25, 4, data).unwrap();
        let location = at(40.99, -71.99);

        let sample = grid.sample(&location, SampleMethod::BilinearEnergy);
        assert_eq!(sample.value, None);
        assert_eq!(sample.source, SampleSource::Missing);

        let sample = grid
            .with_max_fallback_km(120.0)
            .sample(&location, SampleMethod::BilinearEnergy);
        assert_eq!(sample.value, Some(1.5));
        match sample.source {
            SampleSource::NearestSeaCell { distance_km } => {
                assert!(distance_km > 100.0 && distance_km < 110.0)
            }
            other => panic!("unexpected source {other:?}"),
        }
    }

    #[test]
    fn test_nearest() {
        let grid = small_grid(288.0);
        let sample = grid.sample(&at(40.1, -69.2), SampleMethod::Nearest);
        assert_eq!(sample.value, Some(23.0));
        assert_eq!(sample.source, SampleSource::NearestCell);
    }
}
