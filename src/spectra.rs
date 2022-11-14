use std::{ops::Mul, f64::consts::PI};

use contour::{Contour, ContourBuilder};
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use itertools::Itertools;
use kdtree::{KdTree, distance::squared_euclidean};
use serde::{Deserialize, Serialize};

use crate::{
    swell::{SwellProviderError, SwellSummary},
    tools::{
        analysis::{lerp, watershed, WatershedError, bilerp},
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
    direction: Vec<f64>,
    /// Energy values in m2/hz/rad
    pub energy: Vec<f64>,
    /// Direction Convention
    pub dir_convention: DirectionConvention,
}

impl Spectra {
    pub fn new(
        frequency: Vec<f64>,
        direction: Vec<f64>,
        values: Vec<f64>,
        dir_convention: DirectionConvention,
    ) -> Self {
        Spectra {
            frequency,
            direction,
            energy: values,
            dir_convention,
        }
    }

    /// Period bins
    pub fn period(&self) -> Vec<f64> {
        self.frequency.iter().map(|f| 1.0 / f).collect()
    }

    /// Direction bins normalized to DirectionConvention::From in degrees
    pub fn direction_deg(&self) -> Vec<f64> {
        self.direction
            .iter()
            .map(|d| self.dir_convention.normalize(d.to_degrees()))
            .collect()
    }

    /// Direction bins normalized to DirectionContention::From in radians
    pub fn direction_rad(&self) -> Vec<f64> {
        self.direction_deg().iter().map(|d| d.to_radians()).collect()
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
    /// Used by the contour generator that does smoothing on its own 
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

    /// Interpolated frequency index bounds for a given frequency
    pub fn closest_k(&self, freq: f64) -> (usize, usize) {
        let lower = self.frequency
            .iter()
            .position(|f| f.le(&freq))
            .unwrap_or(self.frequency.len() - 1);
        
        if lower == self.frequency.len() - 1 {
            (lower, lower)
        } else {
            (lower, lower + 1)
        }
    }

    /// Interpolated direection index bounds for a given direction
    pub fn closest_th(&self, dir: f64) -> (usize, usize) {
        let lower = self.direction
            .iter()
            .position(|d| d.le(&dir))
            .unwrap_or(0);
        
        if lower == self.direction.len() - 1 {
            // Direction wraps around cuz its a circle
            (lower, 0)
        } else {
            (lower, lower + 1)
        }
    }

    pub fn energy_indices(&self) -> Vec<(usize, usize)> {
        let nk = self.nk();
        (0..self.energy.len()).map(|i| {
            let ik = i % nk;
            let ith = i / nk;
            (ik, ith)
        }).collect()
    }

    /// Get the energy for a given frequency and direction index
    pub fn energy_at(&self, ik: usize, ith: usize) -> f64 {
        let isp = ik + (ith * self.frequency.len());
        self.energy[isp]
    }

    /// Interpolated energy for an arbitrary frequency and direction combo
    pub fn interp_energy(&self, freq: f64, dir: f64) -> f64 {

        let (x1, x2) = self.closest_k(freq);
        let (y1, y2) = self.closest_th(dir);

        let f1 = self.frequency[x1];
        let f2 = self.frequency[x2];
        let x_diff = (freq - f1) / (f2 - f1);

        let d1 = self.direction[y1];
        let d2 = self.direction[y2];
        let y_diff = (dir - d1) / (d2 - d1);

        let a = self.energy_at(x1, y1);
        let b = self.energy_at(x2, y1);
        let c = self.energy_at(x1, y2);
        let d = self.energy_at(x2, y2);

        println!("============");
        // println!("freq {freq}");
        println!("dir {dir}");
        // println!("a({f1},{d1}) {a}");
        // println!("b({f2},{d1}) {b}");
        // println!("c({f1},{d2}) {c}");
        // println!("d({f2},{d2}) {d}");

        let e = bilerp(a, b, c, d, x_diff, y_diff);

        println!("e {e}");

        e
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
            &self.dir_convention,
        );

        Ok(SwellSummary {
            summary,
            components,
        })
    }

    /// Projects the energy data to cartesian coordinates
    pub fn project_cartesian(&self, size: usize, period_threshold: Option<f64>, exp_scale: Option<f64>) -> Vec<f64> {
        let directions = self.direction_deg();
        let periods = self.period();

        // If 0, 0 is the upper left corner
        let origin = (size / 2, size / 2);
        let max_period = periods
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let exp_scale = exp_scale.unwrap_or(1.0);
        let period_threshold = period_threshold.unwrap_or(*max_period);
        let period_scale_threshold = period_threshold.powf(exp_scale); 

        // Build the kdtree of the cartesian coordinates for all of the points that we have 
        let mut kdtree = KdTree::new(2);
        self.energy_indices()
            .iter()
            .enumerate()
            .for_each(|(i, (ik, ith))| {
                if periods[*ik] > period_threshold {
                    return;
                }
                
                let r = ((size / 2) as f64) * (periods[*ik].powf(exp_scale) / period_scale_threshold);
                let t = (directions[*ith] + 270.0) % 360.0;
                let x = (origin.0 as f64) + (r * t.to_radians().cos());
                let y = (origin.1 as f64) + (r * t.to_radians().sin());
                let p = [x, y];
                let _  = kdtree.add(p, i);
            });

        // Create a new image of the specified sizing, and map the pixels to the 
        // energy data using the kdtree representation
        let mut cartesian = vec![0.0; size * size];
        cartesian
            .iter_mut()
            .enumerate()
            .for_each(|(i, ce)| {
                let x = (i % size) as f64; 
                let y = (i / size) as f64;
                let p = [x, y];

                let r = y.atan2(x);
                if r > size as f64 {
                    *ce = f64::NAN;
                    return;
                }

                let Ok(nearest) = kdtree.nearest(&p, 1, &squared_euclidean) else {
                    *ce = f64::NAN;
                    return;
                };

                let nearest_i = nearest[0].1;
                let nearest_energy = self.energy[*nearest_i];
                *ce = nearest_energy;
            });

        cartesian
    } 

    /// Contours
    pub fn contoured(&self) -> Result<GeoJson, ContourError> {
        let c = ContourBuilder::new(self.nk() as u32, self.nth() as u32, true);

        let (min, max) = self.energy_range();
        let t = linspace(0.10, max, 10).collect::<Vec<f64>>();

        let contours = c
            .contours(&self.energy, &t)
            .map_err(|_| ContourError::ContourFailure)?;

        let features: Vec<Feature> = contours
            .iter()
            .map(|c| {
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
                                                let y = self
                                                    .dir_convention
                                                    .normalize(self.ith(point[1]).to_degrees());
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