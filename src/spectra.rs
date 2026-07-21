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
        vector::{argsort_partial, diff},
        waves::pt_mean,
    },
    units::direction::DirectionConvention,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpectralAxis {
    Frequency,
    Direction,
}

/// cos-2s exponent cap for directional de-binning: r1 -> 1 sends s -> infinity,
/// and s = 399 is roughly 4 degrees of spread, well below one 5 degree output
/// bin, so narrower fits are indistinguishable.
const DEBIN_S_MAX: f64 = 399.0;

/// De-binned output direction axis: 72 bins of 5 degrees centered at 2.5 + 5k
const DEBIN_DIRECTION_COUNT: usize = 72;
const DEBIN_DIRECTION_STEP: f64 = 5.0;

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

    /// Per-frequency circular first moments of the binned directional
    /// distribution as (m0 in m2/hz, mean direction in degrees, resultant
    /// length r1). The first moments survive direction binning with sub-bin
    /// precision, which is what makes de-binning possible.
    fn binned_first_moments(&self) -> Vec<(f64, f64, f64)> {
        let direction = self.direction_deg();
        let widths = circular_bin_widths_deg(&direction);
        let nk = self.nk();
        let nth = self.nth();

        (0..nk)
            .map(|ik| {
                let mut m0 = 0.0;
                let mut esin = 0.0;
                let mut ecos = 0.0;
                for ith in 0..nth {
                    let energy = self.energy_at(ik, ith);
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
                    let mean_direction = b1.atan2(a1).to_degrees().rem_euclid(360.0);
                    (m0, mean_direction, a1.hypot(b1))
                }
            })
            .collect()
    }

    /// De-bin the directional distribution onto a smooth 72 x 5 degree axis.
    ///
    /// Binned direction axes (like 36 x ~10 degree buoy spectra) render as a
    /// directional staircase, which measurably shifts island shadows for
    /// narrow long-period swell when fed to SWAN as a boundary. For every
    /// frequency row this fits a cos-2s spreading shape to the circular first
    /// moment of the binned distribution and samples it on the 5 degree axis,
    /// preserving each row's total energy exactly and its mean direction and
    /// directional spread to sub-bin precision. Note `interpolate_to_grid` is
    /// not a substitute: interpolating the staircase preserves the staircase.
    ///
    /// Known limitation of the per-frequency fit: two wave systems crossing
    /// at the same frequency from different directions merge into a single
    /// cos-2s shape.
    pub fn debin(&self) -> Spectra {
        self.debin_rows(self.binned_first_moments())
    }

    /// De-bin like [`Spectra::debin`], but override the per-frequency mean
    /// direction (degrees, from-convention) and first normalized polar
    /// coefficient r1 with exact values where available - for example NDBC
    /// alpha1 (.swdir) and r1 (.swr1) records for spectra reconstructed from
    /// directional Fourier coefficients. Rows with missing or invalid values
    /// (such as the NDBC 999.0 fill) fall back to moments estimated from the
    /// bins; each row's total energy always comes from the binned spectrum.
    pub fn debin_with_moments(
        &self,
        mean_direction: &[f64],
        first_polar_coefficient: &[f64],
    ) -> Spectra {
        let mut moments = self.binned_first_moments();
        for (ik, moment) in moments.iter_mut().enumerate() {
            let (Some(direction), Some(r1)) =
                (mean_direction.get(ik), first_polar_coefficient.get(ik))
            else {
                continue;
            };
            if (0.0..=360.0).contains(direction) && (0.0..=1.0).contains(r1) {
                *moment = (moment.0, *direction, *r1);
            }
        }
        self.debin_rows(moments)
    }

    fn debin_rows(&self, moments: Vec<(f64, f64, f64)>) -> Spectra {
        let nk = self.nk();
        let direction: Vec<f64> = (0..DEBIN_DIRECTION_COUNT)
            .map(|ith| DEBIN_DIRECTION_STEP / 2.0 + DEBIN_DIRECTION_STEP * ith as f64)
            .collect();

        let mut energy = vec![0.0; nk * DEBIN_DIRECTION_COUNT];
        for (ik, (m0, mean_direction, r1)) in moments.into_iter().enumerate() {
            if m0 <= 0.0 {
                continue;
            }

            // Compute the cos-2s shape in log space so large exponents do not
            // underflow before normalization
            let s = (r1 / (1.0 - r1).max(1.0 / (1.0 + DEBIN_S_MAX))).min(DEBIN_S_MAX);
            let log_shape: Vec<f64> = direction
                .iter()
                .map(|d| {
                    let half = ((d - mean_direction) / 2.0).to_radians();
                    2.0 * s * half.cos().abs().max(1.0e-300).ln()
                })
                .collect();
            let log_max = log_shape.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let shape: Vec<f64> = log_shape.iter().map(|l| (l - log_max).exp()).collect();

            // Discrete normalization preserves the row's m0 exactly
            let scale = shape.iter().sum::<f64>() * DEBIN_DIRECTION_STEP.to_radians();
            for (ith, value) in shape.iter().enumerate() {
                energy[ik + ith * nk] = m0 * (value / scale);
            }
        }

        Spectra::new(
            self.frequency.clone(),
            direction.iter().map(|d| d.to_radians()).collect(),
            energy,
            DirectionConvention::From,
        )
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
    pub fn project_cartesian_with_map(
        &self,
        target: &[f64],
        map: &CartesianProjectionMap,
    ) -> Vec<f64> {
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

/// Circular bin widths in degrees for a direction axis in degrees, in the
/// same order as `direction`: half the gap to each neighbor on the circle,
/// wrapping around 360. Real buoy axes are non-uniform (36 integer
/// directions alternating 9 and 11 degree gaps), so widths cannot be assumed
/// constant, and axes are not assumed pre-sorted (e.g. a Met-convention axis
/// normalized via `270 - dir` reverses ascending input order) - this sorts
/// internally so neighbor gaps are always spatial neighbors on the circle.
fn circular_bin_widths_deg(direction: &[f64]) -> Vec<f64> {
    let count = direction.len();
    let order = argsort_partial(direction);
    let sorted: Vec<f64> = order.iter().map(|&i| direction[i]).collect();

    let mut widths = vec![0.0; count];
    for (position, &original_index) in order.iter().enumerate() {
        let up = (sorted[(position + 1) % count] - sorted[position]).rem_euclid(360.0);
        let down =
            (sorted[position] - sorted[(position + count - 1) % count]).rem_euclid(360.0);
        widths[original_index] = (up + down) / 2.0;
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The playbuoy direction axis: 36 integer directions alternating 9 and
    /// 11 degree gaps
    fn playbuoy_directions_deg() -> Vec<f64> {
        (0..36)
            .map(|i| 6.0 + 20.0 * ((i / 2) as f64) + 9.0 * ((i % 2) as f64))
            .collect()
    }

    /// A binned spectrum on the playbuoy axis: one narrow row, one broad row,
    /// and one zero row
    fn binned_spectra() -> Spectra {
        let direction_deg = playbuoy_directions_deg();
        let frequency = vec![0.07, 0.1, 0.2];
        let mut energy = vec![0.0; frequency.len() * direction_deg.len()];
        for (ith, dir) in direction_deg.iter().enumerate() {
            let narrow = (-((dir - 115.0) / 12.0).powi(2)).exp();
            let broad = 0.4 * (-((dir - 240.0) / 55.0).powi(2)).exp();
            energy[0 + ith * frequency.len()] = narrow;
            energy[1 + ith * frequency.len()] = broad;
        }
        Spectra::new(
            frequency,
            direction_deg.iter().map(|d| d.to_radians()).collect(),
            energy,
            DirectionConvention::From,
        )
    }

    fn spread_deg(r1: f64) -> f64 {
        (2.0 * (1.0 - r1)).max(0.0).sqrt().to_degrees()
    }

    #[test]
    fn circular_bin_widths_wrap_and_cover_the_circle() {
        let widths = circular_bin_widths_deg(&playbuoy_directions_deg());
        assert!((widths.iter().sum::<f64>() - 360.0).abs() < 1.0e-9);

        // An asymmetric axis: bin widths are the midpoint-to-midpoint spans
        let widths = circular_bin_widths_deg(&[0.0, 90.0, 180.0]);
        assert_eq!(widths, vec![135.0, 90.0, 135.0]);
    }

    #[test]
    fn circular_bin_widths_are_order_independent() {
        // A reversed, non-monotonic axis (as produced by e.g. Met convention
        // normalization, which computes 270 - dir) must give each direction
        // the same width as the sorted axis, keyed by its own position in
        // the input rather than assuming ascending order
        let sorted = [0.0, 90.0, 180.0, 270.0];
        let reversed = [270.0, 180.0, 90.0, 0.0];

        let sorted_widths = circular_bin_widths_deg(&sorted);
        let reversed_widths = circular_bin_widths_deg(&reversed);

        for (i, &dir) in reversed.iter().enumerate() {
            let sorted_index = sorted.iter().position(|&d| d == dir).unwrap();
            assert_eq!(reversed_widths[i], sorted_widths[sorted_index]);
        }
    }

    #[test]
    fn debin_round_trips_row_moments_with_met_convention() {
        // Met-convention directions normalize via `270 - dir`, which reverses
        // ascending input into a descending physical axis; debin must still
        // recover the correct moments rather than treating array order as
        // spatial order
        let direction_met_deg: Vec<f64> = (0..36).map(|i| 10.0 * i as f64).collect();
        let frequency = vec![0.07, 0.1];
        let mut energy = vec![0.0; frequency.len() * direction_met_deg.len()];
        for (ith, met_dir) in direction_met_deg.iter().enumerate() {
            let physical_dir = DirectionConvention::Met.normalize(*met_dir);
            let narrow = (-((physical_dir - 200.0) / 12.0).powi(2)).exp();
            energy[0 + ith * frequency.len()] = narrow;
        }
        let binned = Spectra::new(
            frequency,
            direction_met_deg.iter().map(|d| d.to_radians()).collect(),
            energy,
            DirectionConvention::Met,
        );

        let debinned = binned.debin();
        let source_moments = binned.binned_first_moments();
        let debinned_moments = debinned.binned_first_moments();
        for ((m0_in, dir_in, r1_in), (m0_out, dir_out, r1_out)) in
            source_moments.into_iter().zip(debinned_moments)
        {
            if m0_in <= 0.0 {
                assert_eq!(m0_out, 0.0);
                continue;
            }
            assert!((m0_out / m0_in - 1.0).abs() < 1.0e-9);
            let dir_error = (dir_out - dir_in + 180.0).rem_euclid(360.0) - 180.0;
            assert!(dir_error.abs() < 0.5, "dir_in={dir_in} dir_out={dir_out}");
            assert!((spread_deg(r1_out) - spread_deg(r1_in)).abs() < 0.5);
        }
    }

    #[test]
    fn debin_round_trips_row_moments() {
        let binned = binned_spectra();
        let debinned = binned.debin();

        assert_eq!(debinned.nk(), binned.nk());
        assert_eq!(debinned.nth(), DEBIN_DIRECTION_COUNT);
        assert_eq!(debinned.direction_deg()[0], 2.5);
        assert_eq!(debinned.direction_deg()[71], 357.5);

        let source_moments = binned.binned_first_moments();
        let debinned_moments = debinned.binned_first_moments();
        for ((m0_in, dir_in, r1_in), (m0_out, dir_out, r1_out)) in
            source_moments.into_iter().zip(debinned_moments)
        {
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

    #[test]
    fn debin_with_moments_prefers_exact_coefficients() {
        let binned = binned_spectra();
        // Exact moments for the first two rows; the NDBC 999.0 fill for the
        // zero row must fall back to the bin estimate
        let mean_direction = vec![120.0, 250.0, 999.0];
        let first_polar_coefficient = vec![0.95, 0.5, 999.0];

        let debinned = binned.debin_with_moments(&mean_direction, &first_polar_coefficient);
        let moments = debinned.binned_first_moments();

        let source_moments = binned.binned_first_moments();
        for ik in 0..2 {
            let (m0, direction, r1) = moments[ik];
            assert!((m0 / source_moments[ik].0 - 1.0).abs() < 1.0e-12);
            let dir_error = (direction - mean_direction[ik] + 180.0).rem_euclid(360.0) - 180.0;
            assert!(dir_error.abs() < 0.01);
            assert!((spread_deg(r1) - spread_deg(first_polar_coefficient[ik])).abs() < 0.01);
        }
        assert_eq!(moments[2], (0.0, 0.0, 0.0));
    }

    #[test]
    fn debin_zeroes_rows_with_non_finite_energy() {
        let mut binned = binned_spectra();
        let nk = binned.nk();
        for ith in 0..binned.nth() {
            binned.energy[2 + ith * nk] = f64::NAN;
        }

        let debinned = binned.debin();
        for ith in 0..debinned.nth() {
            assert_eq!(debinned.energy_at(2, ith), 0.0);
        }
    }
}
