use geojson::{FeatureCollection, GeoJson};
use kdtree::{distance::squared_euclidean, KdTree};
use serde::{Deserialize, Serialize};

use crate::{
    swell::{SwellProviderError, SwellSummary},
    tools::{
        analysis::{bilerp, lerp, watershed, WatershedError},
        contour::{compute_contours, ContourError},
        interpolation::{circular_pchip_interpolate, PchipInterpolator},
        linspace::linspace,
        vector::diff,
        waves::pt_mean,
    },
    units::direction::DirectionConvention,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpectralAxis {
    Frequency,
    Direction,
}

/// Pre-computed mapping from cartesian pixel indices to spectral indices.
/// Used to accelerate repeated calls to `project_cartesian_with_map` when
/// the frequency/direction grid remains constant across multiple spectra.
#[derive(Clone, Debug)]
pub struct CartesianProjectionMap {
    /// The size of the cartesian projection (size x size pixels)
    pub size: usize,
    /// Mapping from pixel index to spectral index (None for pixels outside the valid region)
    pub indices: Vec<Option<usize>>,
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

    /// Given a swell partition map, return the spectra for the given component
    /// with the energy of all other component members set to 0
    pub fn from_component(
        source_spectra: &Spectra,
        components: &(Vec<i32>, usize),
        id: i32,
    ) -> Spectra {
        let component_energy = source_spectra
            .energy
            .iter()
            .zip(components.0.iter())
            .map(|(e, i)| if i == &id { *e } else { 0.0 })
            .collect::<Vec<f64>>();

        Spectra::new(
            source_spectra.frequency.clone(),
            source_spectra.direction.clone(),
            component_energy,
            source_spectra.dir_convention.clone(),
        )
    }

    /// Given a swell partition map, return the spectra for all components
    /// with the energy of all other component members set to 0 for each
    /// component
    pub fn split_from_components(
        source_spectra: &Spectra,
        components: &(Vec<i32>, usize),
        limit: Option<usize>,
    ) -> Vec<Spectra> {
        let limit = limit.unwrap_or(components.1);

        (0..limit)
            .map(|i| Spectra::from_component(source_spectra, components, i as i32))
            .collect()
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
        self.direction_deg()
            .iter()
            .map(|d| d.to_radians())
            .collect()
    }

    /// The raw direction bins in radians
    pub fn direction_raw(&self) -> Vec<f64> {
        self.direction.clone()
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

        let v_lower = self.frequency[i_lower as usize];
        let v_upper = self.frequency[i_upper as usize];
        lerp(&v_lower, &v_upper, &f_index, &i_lower, &i_upper)
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

        let v_lower = self.direction[i_lower as usize];
        let v_upper = self.direction[i_upper as usize];
        lerp(&v_lower, &v_upper, &d_index, &i_lower, &i_upper)
    }

    /// Interpolated frequency index bounds for a given frequency
    pub fn closest_k(&self, freq: f64) -> (usize, usize) {
        let lower = self
            .frequency
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
        let lower = self.direction.iter().position(|d| d.le(&dir)).unwrap_or(0);

        if lower == self.direction.len() - 1 {
            // Direction wraps around cuz its a circle
            (lower, 0)
        } else {
            (lower, lower + 1)
        }
    }

    pub fn energy_indices(&self) -> Vec<(usize, usize)> {
        let nk = self.nk();
        (0..self.energy.len())
            .map(|i| {
                let ik = i % nk;
                let ith = i / nk;
                (ik, ith)
            })
            .collect()
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

        let d1 = self.direction[y1];
        let d2 = self.direction[y2];

        let a = self.energy_at(x1, y1);
        let b = self.energy_at(x2, y1);
        let c = self.energy_at(x1, y2);
        let d = self.energy_at(x2, y2);

        bilerp(&a, &b, &c, &d, &freq, &f1, &f2, &dir, &d1, &d2)
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

    /// Calculate the given frequency moment i
    pub fn mom_f(&self, mom_i: i32) -> Vec<f64> {
        let nth = self.nth();
        let nk = self.nk();
        let dk = self.dk();

        let mut moment = vec![0.0; nth];

        for ik in 0..nk {
            let fp = self.frequency[ik].powi(mom_i);
            for ith in 0..nth {
                moment[ith] += fp * self.energy_at(ik, ith) * dk[ik];
            }
        }

        moment
    }

    /// Calculate the given directional moment i
    pub fn mom_d(&self, mom_i: i32) -> Vec<(f64, f64)> {
        let nk = self.nk();
        let nth = self.nth();
        let dth = self.dth();

        let mut moment = vec![(0.0, 0.0); nk];

        for ith in 0..nth {
            let cs = self.direction[ith].cos().powi(mom_i);
            let ss = self.direction[ith].sin().powi(mom_i);
            for ik in 0..nk {
                let mv = dth[ith] * self.energy_at(ik, ith);
                moment[ik].0 += mv * ss;
                moment[ik].1 += mv * cs;
            }
        }

        moment
    }

    /// Calculate the mean wave direction for every frequency point
    pub fn mean_wave_direction_f(&self) -> Vec<f64> {
        let momd = self.mom_d(1);

        momd.iter()
            .map(|(esin, ecos)| {
                let dm = esin.atan2(*ecos).to_degrees();
                match self.dir_convention {
                    DirectionConvention::Met => (270.0 - dm) % 360.0,
                    DirectionConvention::From => (360.0 + dm) % 360.0,
                    DirectionConvention::Towards => (180.0 + dm) % 360.0,
                }
            })
            .collect()
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
    pub fn partition(
        &self,
        levels: usize,
        blur: Option<f32>,
    ) -> Result<(Vec<i32>, usize), WatershedError> {
        watershed(
            &self.energy,
            self.frequency.len(),
            self.direction.len(),
            levels,
            blur,
        )
    }

    /// Extract swell components
    pub fn swell_data(
        &self,
        depth: Option<f64>,
        wind_speed: Option<f64>,
        wind_direction: Option<f64>,
        partitions: &(Vec<i32>, usize),
    ) -> Result<crate::swell::SwellSummary, SwellProviderError> {
        let (imo, partition_count) = partitions;
        let (summary, components) = pt_mean(
            *partition_count,
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
    ///
    /// Pre-compute the cartesian projection mapping from pixel indices to spectral indices.
    /// This mapping can be reused across multiple calls to `project_cartesian_with_map` when
    /// projecting different data (e.g., energy, partitions) that share the same frequency/direction grid.
    ///
    /// # Arguments
    /// * `size` - The size of the cartesian projection in pixels
    /// * `period_threshold` - The maximum period to project. This is used to filter out the longer period swell
    /// * `exp_scale` - The exponent to use for scaling the period. This is used to make the longer period swell more visible
    ///
    /// # Returns
    /// * A `CartesianProjectionMap` containing the pre-computed pixel-to-spectral-index mapping
    pub fn compute_cartesian_projection_map(
        &self,
        size: usize,
        period_threshold: Option<f64>,
        exp_scale: Option<f64>,
    ) -> CartesianProjectionMap {
        let directions = self.direction_deg();
        let periods = self.period();

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

                let r =
                    ((size / 2) as f64) * (periods[*ik].powf(exp_scale) / period_scale_threshold);
                let t = (directions[*ith] + 270.0) % 360.0;
                let x = (origin.0 as f64) + (r * t.to_radians().cos());
                let y = (origin.1 as f64) + (r * t.to_radians().sin());
                let p = [x, y];
                let _ = kdtree.add(p, i);
            });

        // Pre-compute the mapping from each pixel to its nearest spectral index
        let indices: Vec<Option<usize>> = (0..size * size)
            .map(|i| {
                let x = (i % size) as f64;
                let y = (i / size) as f64;
                let p = [x, y];

                let r = y.atan2(x);
                if r > size as f64 {
                    return None;
                }

                kdtree
                    .nearest(&p, 1, &squared_euclidean)
                    .ok()
                    .and_then(|nearest| nearest.first().map(|(_, idx)| **idx))
            })
            .collect();

        CartesianProjectionMap { size, indices }
    }

    /// Project target data to cartesian coordinates using a pre-computed projection map.
    /// This is much faster than `project_cartesian` when projecting multiple datasets
    /// that share the same frequency/direction grid.
    ///
    /// # Arguments
    /// * `target` - The target data to project (must be same size as spectra energy)
    /// * `map` - The pre-computed projection map from `compute_cartesian_projection_map`
    ///
    /// # Returns
    /// * A vector of the projected data
    pub fn project_cartesian_with_map(&self, target: &[f64], map: &CartesianProjectionMap) -> Vec<f64> {
        map.indices
            .iter()
            .map(|idx| match idx {
                Some(i) => target[*i],
                None => f64::NAN,
            })
            .collect()
    }

    /// # Arguments
    /// * `target` - The target energy data to project. This is usually the energy data of the swell component,
    ///             but can be any data of the same size as the spectra
    /// * `size` - The size of the cartesian projection in pixels
    /// * `period_threshold` - The maximum period to project. This is used to filter out the longer period swell
    /// * `exp_scale` - The exponent to use for scaling the period. This is used to make the longer period swell more visible
    ///
    /// # Returns
    /// * A vector of the projected energy data
    pub fn project_cartesian(
        &self,
        target: &[f64],
        size: usize,
        period_threshold: Option<f64>,
        exp_scale: Option<f64>,
    ) -> Vec<f64> {
        let map = self.compute_cartesian_projection_map(size, period_threshold, exp_scale);
        self.project_cartesian_with_map(target, &map)
    }

    /// Contours
    pub fn contoured(&self) -> Result<GeoJson, ContourError> {
        let (_min, max) = self.energy_range();
        let t = linspace(0.10, max, 10).collect::<Vec<f64>>();

        let features = compute_contours(
            &self.energy,
            self.nk(),
            self.nth(),
            &t,
            Some(|point: &Vec<f64>| {
                let x = 1.0 / self.ik(point[0]);
                let y = self
                    .dir_convention
                    .normalize(self.ith(point[1]).to_degrees());
                vec![x, y]
            }),
            None::<Box<dyn Fn(&usize, &f64) -> String>>,
        )?;

        Ok(GeoJson::from(FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }))
    }

    /// Interpolate spectra to a new frequency/direction grid.
    ///
    /// Uses log-frequency PCHIP interpolation and circular direction
    /// interpolation to preserve energy distribution and handle the
    /// periodic nature of directions.
    ///
    /// # Arguments
    /// * `target_freq` - Target frequency bins in Hz (must be sorted ascending)
    /// * `target_dir` - Target direction bins in degrees (must be sorted ascending, 0-360 range)
    ///
    /// # Returns
    /// A new `Spectra` with energy interpolated to the target grid.
    ///
    /// # Behavior
    /// - Frequencies below source minimum are zero-filled
    /// - Frequencies above source maximum use boundary clamping
    /// - Directions use circular interpolation (360° wraps to 0°)
    /// - Negative energy values are clamped to zero
    /// - Source directions are automatically sorted before interpolation
    pub fn interpolate_to_grid(&self, target_freq: &[f64], target_dir: &[f64]) -> Spectra {
        let nk_src = self.nk();
        let nth_src = self.nth();
        let nk_tgt = target_freq.len();
        let nth_tgt = target_dir.len();

        // Step 1: Get source directions in degrees and sort them
        let src_dir_deg = self.direction_deg();

        // Create sort indices
        let mut sort_indices: Vec<usize> = (0..nth_src).collect();
        sort_indices.sort_by(|&a, &b| {
            src_dir_deg[a]
                .partial_cmp(&src_dir_deg[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply sort to directions
        let sorted_dir: Vec<f64> = sort_indices.iter().map(|&i| src_dir_deg[i]).collect();

        // Reorder energy by sorted direction indices
        // Energy layout: energy[ik + ith * nk] where ik=freq_idx, ith=dir_idx
        let mut sorted_energy = vec![0.0; nk_src * nth_src];
        for ik in 0..nk_src {
            for (new_ith, &old_ith) in sort_indices.iter().enumerate() {
                let old_idx = ik + old_ith * nk_src;
                let new_idx = ik + new_ith * nk_src;
                sorted_energy[new_idx] = self.energy[old_idx];
            }
        }

        // Step 2: Transform frequencies to log-space
        let log_src_freq: Vec<f64> = self.frequency.iter().map(|f| f.ln()).collect();
        let log_tgt_freq: Vec<f64> = target_freq.iter().map(|f| f.ln()).collect();

        // Step 3: Interpolate along frequency for each source direction
        // Result shape: (nk_tgt, nth_src) stored as freq_interp[ik_tgt + ith_src * nk_tgt]
        let mut freq_interp = vec![0.0; nk_tgt * nth_src];

        let src_freq_min = self.frequency[0];

        for ith in 0..nth_src {
            // Extract energy slice for this direction
            let energy_slice: Vec<f64> = (0..nk_src)
                .map(|ik| sorted_energy[ik + ith * nk_src])
                .collect();

            // Create PCHIP interpolator in log-freq space
            let pchip = PchipInterpolator::new(&log_src_freq, &energy_slice);

            // Interpolate to target frequencies
            for (ik_tgt, &log_f) in log_tgt_freq.iter().enumerate() {
                let f = log_f.exp();
                let value = if f < src_freq_min {
                    // Below source range - zero fill
                    0.0
                } else {
                    pchip.interpolate(log_f)
                };
                freq_interp[ik_tgt + ith * nk_tgt] = value;
            }
        }

        // Step 4: Interpolate along direction (circular) for each target frequency
        // Result shape: (nk_tgt, nth_tgt) stored as result[ik_tgt + ith_tgt * nk_tgt]
        let mut result_energy = vec![0.0; nk_tgt * nth_tgt];

        for ik_tgt in 0..nk_tgt {
            // Extract values along direction for this frequency
            let values: Vec<f64> = (0..nth_src)
                .map(|ith| freq_interp[ik_tgt + ith * nk_tgt])
                .collect();

            // Interpolate to each target direction
            for (ith_tgt, &tgt_d) in target_dir.iter().enumerate() {
                let value = circular_pchip_interpolate(&sorted_dir, &values, tgt_d);
                let idx = ik_tgt + ith_tgt * nk_tgt;
                // Clamp negative values to zero
                result_energy[idx] = value.max(0.0);
            }
        }

        // Step 5: Create result Spectra
        // Convert target directions from degrees to radians
        let target_dir_rad: Vec<f64> = target_dir.iter().map(|d| d.to_radians()).collect();

        Spectra::new(
            target_freq.to_vec(),
            target_dir_rad,
            result_energy,
            DirectionConvention::From,
        )
    }
}
