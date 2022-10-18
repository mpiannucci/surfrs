use contour::{Contour, ContourBuilder};
use geojson::{Feature, FeatureCollection, GeoJson, Value, Geometry};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    swell::{SwellProviderError, SwellSummary},
    tools::{
        analysis::{watershed, WatershedError, lerp},
        linspace::linspace,
        vector::diff,
        waves::pt_mean,
    },
    units::direction::DirectionConvention,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContourError {
    ContourFailure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpectralAxis {
    Frequency,
    Direction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spectra {
    /// Frequency bins in hz
    pub frequency: Vec<f64>,
    /// Direction bins in rad
    pub direction: Vec<f64>,
    /// Energy values in m2/hz/rad
    pub energy: Vec<f64>,
}

impl Spectra {
    pub fn new(frequency: Vec<f64>, direction: Vec<f64>, values: Vec<f64>) -> Self {
        Spectra {
            frequency,
            direction,
            energy: values,
        }
    }

    /// Number of frequency bins
    pub fn nk(&self) -> usize {
        self.frequency.len()
    }

    /// Vector of frequency bandwidths
    pub fn dk(&self) -> Vec<f64> {
        diff(&self.frequency)
    }

    /// Number of directional bins
    pub fn nth(&self) -> usize {
        self.direction.len()
    }

    /// Vector of directional bandwidths
    pub fn dth(&self) -> Vec<f64> {
        diff(&self.direction)
    }

    /// Interpolated frequency for a given index
    pub fn ik(&self, f_index: f64) -> f64 {
        let i_lower = f_index.floor();
        let i_upper = f_index.ceil();

        if i_upper >= self.nk() as f64 {
            return self.frequency[self.frequency.len() - 1];
        }

        if i_lower < 0.0 {
            return self.frequency[0];
        }

        let frac = (f_index - i_lower) / (i_upper - i_lower);

        let v_lower = self.frequency[i_lower as usize];
        let v_upper = self.frequency[i_upper as usize];
        lerp(&v_lower, &v_upper, &frac)
    }

    /// Interpolated direction for a given index
    pub fn ith(&self, d_index: f64) -> f64 {
        let i_lower = d_index.floor();
        let i_upper = d_index.ceil();

        if i_upper >= self.nth() as f64 {
            return self.direction[self.direction.len() - 1];
        }

        if i_lower < 0.0 {
            return self.direction[0];
        }

        let frac = (d_index - i_lower) / (i_upper - i_lower);

        let v_lower = self.direction[i_lower as usize];
        let v_upper = self.direction[i_upper as usize];
        lerp(&v_lower, &v_upper, &frac)
    }

    /// One dimensional representation of the energy across the given axis
    /// Result is in m2/hz for SpectralAxis::Frequency or m2/rad for SpectralAxis::Direction
    pub fn oned(&self, axis: SpectralAxis) -> Vec<f64> {
        let nk = self.nk();
        let nth = self.nth();

        match axis {
            SpectralAxis::Frequency => {
                let dth = self.dth();

                let mut oned = vec![0.0; nk];
                for ik in 0..nk {
                    for ith in 0..nth {
                        let i = ik + (ith * nk);
                        oned[ik] += dth[ith] * self.energy[i];
                    }
                }
                oned
            }
            SpectralAxis::Direction => {
                let dk = self.dk();

                let mut oned = vec![0.0; nth];
                for ith in 0..nth {
                    for ik in 0..nk {
                        let i = ik + (ith * nk);
                        oned[ith] += dk[ik] * self.energy[i];
                    }
                }

                oned
            }
        }
    }

    /// The value range of the energy data in the form of (min, max)
    pub fn energy_range(&self) -> (f64, f64) {
        let min = self
            .energy
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max = self
            .energy
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        (*min, *max)
    }

    /// Partition the energy data into discrete swell components
    pub fn partition(&self, levels: usize) -> Result<(Vec<i32>, usize), WatershedError> {
        watershed(
            &self.energy,
            self.frequency.len(),
            self.direction.len(),
            levels,
        )
    }

    /// Extract swell components
    pub fn swell_data(
        &self,
        depth: Option<f64>,
        wind_speed: Option<f64>,
        wind_direction: Option<f64>,
        source_direction_convention: DirectionConvention,
    ) -> Result<crate::swell::SwellSummary, SwellProviderError> {
        let (imo, partition_count) = match watershed(
            &self.energy,
            self.frequency.len(),
            self.direction.len(),
            100,
        ) {
            Ok(result) => Ok(result),
            Err(_) => Err(SwellProviderError::InsufficientData(
                "watershed segmentation of the spectra failed".into(),
            )),
        }?;

        let (summary, components) = pt_mean(
            partition_count,
            &imo,
            &self.frequency,
            &self.direction,
            &self.energy,
            &self.dk(),
            &self.dth(),
            depth,
            wind_speed,
            wind_direction,
            source_direction_convention,
        );

        Ok(SwellSummary {
            summary,
            components,
        })
    }

    /// Contours
    pub fn contoured(&self) -> Result<GeoJson, ContourError> {
        let c = ContourBuilder::new(self.nk() as u32, self.nth() as u32, true);

        let (min, max) = self.energy_range();
        let t = linspace(0.10, max, 10).collect::<Vec<f64>>();

        let contours = c
            .contours(&self.energy, &t)
            .map_err(|_| ContourError::ContourFailure)?;

        let features: Vec<Feature> = contours.iter().map(|c| {
            let mut f = c.to_geojson();
            if let Some(g) = &f.geometry {
                let geo_value: Value = g.value.clone();
                let coords = match geo_value {
                    Value::MultiPolygon(c) => Some(c),
                    _ => None,
                };

                if let Some(c) = coords {
                    let new_coordinates: Vec<Vec<Vec<Vec<f64>>>> = c
                        .iter()
                        .map(|r| {
                            r.iter()
                                .map(|c| {
                                    c.iter()
                                        .map(|point| {
                                            let x = 1.0 / self.ik(point[0]);
                                            let y = self.ith(point[1]);
                                            // let lng = min_lng
                                            //     + (max_lng - min_lng)
                                            //         * (point[0] / (grid.1 as f64));
                                            // let lat = max_lat
                                            //     - (max_lat - min_lat)
                                            //         * (point[1] / (grid.0 as f64));

                                            vec![x, y]
                                        })
                                        .collect()
                                })
                                .collect()
                        })
                        .collect();
                    let new_polygon = Geometry::new(Value::MultiPolygon(new_coordinates));
                    f.geometry = Some(new_polygon);
                }
            }

            f
        })
        .collect::<Vec<Feature>>();

        Ok(GeoJson::from(FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }))
    }
}
