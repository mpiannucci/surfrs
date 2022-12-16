use std::{collections::VecDeque, f64};

use image::imageops;

use crate::tools::{linspace::linspace, vector::argsort};

/// Linearly interpolate between and b by fraction diff
pub fn lerp(a: &f64, b: &f64, diff: &f64) -> f64 {
    a * (1.0 - diff) + (b * diff)
}

/// Bilinearly interpolate
/// Where
///     a = x0y0
///     b = x1y0
///     c = x0y2
///     d = x1y2
/// Adapted from https://stackoverflow.com/a/8661834
pub fn bilerp(a: f64, b: f64, c: f64, d: f64, x_diff: f64, y_diff: f64) -> f64 {
    let l = (a * y_diff) + (c * (1.0 - y_diff));
    let r = (b * y_diff) + (d * (1.0 - y_diff));
    (l * x_diff) + (r * (1.0 - x_diff))
}

/// Converted from MATLAB script at http://billauer.co.il/peakdet.html
///     
/// Returns two arrays
///
/// function [maxtab, mintab]=peakdet(v, delta, x)
/// %PEAKDET Detect peaks in a vector
/// %        [MAXTAB, MINTAB] = PEAKDET(V, DELTA) finds the local
/// %        maxima and minima ("peaks") in the vector V.
/// %        MAXTAB and MINTAB consists of two columns. Column 1
/// %        contains indices in V, and column 2 the found values.
/// %      
/// %        With [MAXTAB, MINTAB] = PEAKDET(V, DELTA, X) the indices
/// %        in MAXTAB and MINTAB are replaced with the corresponding
/// %        X-values.
/// %
/// %        A point is considered a maximum peak if it has the maximal
/// %        value, and was preceded (to the left) by a value lower by
/// %        DELTA.
///
/// % Eli Billauer, 3.4.05 (Explicitly not copyrighted).
/// % This function is released to the public domain; Any use is allowed.
///
pub fn detect_peaks(data: &Vec<f64>, delta: f64) -> (Vec<usize>, Vec<usize>) {
    let mut min_indexes = Vec::new();
    let mut max_indexes = Vec::new();

    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    let mut min_pos: usize = 0;
    let mut max_pos: usize = 0;

    let mut look_for_max = true;

    for (i, v) in data.iter().enumerate() {
        let val = *v;
        if val > max_val {
            max_val = val;
            max_pos = i;
        }
        if val < min_val {
            min_val = val;
            min_pos = i;
        }

        if look_for_max {
            if val < max_val - delta {
                max_indexes.push(max_pos);
                min_val = val;
                min_pos = i;
                look_for_max = false;
            }
        } else {
            if val > min_val + delta {
                min_indexes.push(min_pos);
                max_val = val;
                max_pos = i;
                look_for_max = true;
            }
        }
    }

    (min_indexes, max_indexes)
}

/// Calculate the indexes of a given indexes nearest neighbor cells
/// This is direct port of the routine used by WW3 where typically frequency is the columns and
/// direction is the rows
///
/// 1   2   3   4
/// 5   6   7   8
/// 9   10  11  12
/// 13  14  15  16
///
/// gives
///
/// 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16
///
/// with neighbors calculated for index 5 as
///
/// TODO
///
pub fn nearest_neighbors(width: usize, height: usize, index: usize) -> Vec<usize> {
    let mut neighbors = Vec::new();

    let t = width * height;
    let j = index / width;
    let i = index - (j * width);

    // Point at the left(1)
    if i != 0 {
        neighbors.push(index - 1);
    }

    // Point at the right (2)
    if i != width - 1 {
        neighbors.push(index + 1);
    }

    // Point at the bottom(3)
    if j != 0 {
        neighbors.push(index - width);
    }

    // Point at bottom_wrap to top
    if j == 0 {
        neighbors.push(t - (width - i));
    }

    // Point at the top(4)
    if j != height - 1 {
        neighbors.push(index + width);
    }

    // Point to top_wrap to bottom
    if j == height - 1 {
        neighbors.push(index - (height - 1) * width);
    }

    // Point at the bottom, left(5)
    if i != 0 && j != 0 {
        neighbors.push(index - width);
    }

    // Point at the bottom, left with wrap.
    if i != 0 && j == 0 {
        neighbors.push(index - 1 + width * (height - 1))
    }

    // Point at the bottom, right(6)
    if i != width - 1 && j != 0 {
        neighbors.push(index - width + 1);
    }

    // Point at the bottom, right with wrap
    if i != width - 1 && j == 0 {
        neighbors.push(index + 1 + width * (height - 1));
    }

    // Point at the top, left(7)
    if i != 0 && j != height - 1 {
        neighbors.push(index + width - 1);
    }

    // Point at the top, left with wrap
    if i != 0 && j == height - 1 {
        neighbors.push(index - 1 - width * (height - 1));
    }

    // Point at the top, right(8)
    if i != width - 1 && j != height - 1 {
        neighbors.push(index + width + 1);
    }

    // Point at top, right with wrap
    if i != width - 1 && j == height - 1 {
        neighbors.push(index + 1 - width * (height - 1));
    }

    neighbors
}

#[derive(Debug)]
pub enum WatershedError {
    Unknown,
    InvalidData,
}

/// Implementation of watershed algorithm as used by WW3 in w3partmd.f90
/// More details to come
pub fn watershed(
    data: &[f64],
    width: usize,
    height: usize,
    steps: usize,
    blur: Option<f32>,
) -> Result<(Vec<i32>, usize), WatershedError> {
    let count = width * height;
    if data.len() != count {
        return Err(WatershedError::InvalidData);
    }

    let min_value = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_value = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Scale the data
    let fact = (steps as f64 - 1.0) / (max_value - min_value);

    // Digitize the signal, mapping each energy value to a level from 0 to steps
    // If a blur is specified, apply it 
    let imi = if let Some(blur) = blur {
        let range = max_value - min_value;
        let dat = data
            .iter()
            .map(|v| ((1.0 - (max_value - v) / range) * 255.0) as u8)
            .collect();
        let im = image::GrayImage::from_raw(width as u32, height as u32, dat).unwrap();
        let blurred_im = imageops::blur(&im, blur);

        blurred_im
            .as_raw()
            .iter()
            .map(|v| {
                let scaled_v = min_value + ((*v as f64) / 255.0) * range;
                1u8.max((steps as u8).min((1.0 + (max_value - scaled_v) * fact).round() as u8))
            })
            .collect::<Vec<u8>>()
    } else {
        data.iter()
            .map(|v| 1u8.max((steps as u8).min((1.0 + (max_value - v) * fact).round() as u8)))
            .collect::<Vec<u8>>()
    };

    // Sort the digitized data indices, so all levels are grouped in order
    let ind = argsort::<u8>(&imi);

    // Compute the nearest neighbor for every index ahead of time
    let neigh = (0..count)
        .map(|i| nearest_neighbors(width, height, i))
        .collect::<Vec<Vec<usize>>>();

    // Constants
    const MASK: i32 = -2;
    const INIT: i32 = -1;
    const IWSHED: i32 = 0;
    const IFICT_PIXEL: i32 = -100;

    // Initialize
    let mut ic_label = 0;
    let mut m = 0;
    let mut m_save;
    let mut fifo: VecDeque<i32> = VecDeque::new();
    let mut imo = vec![INIT; count];
    let mut imd = vec![0; count];

    // Iterate the levels looking for the watersheds
    for ih in 1u8..=(steps as u8) {
        m_save = m; // 0

        while m < count {
            let ip = ind[m]; // 471
            if imi[ip] != ih {
                //  imi[ip] = 1
                break;
            }

            // Flag the point, if it stays flagged, it is a separate minimum.
            imo[ip] = MASK;

            // Consider neighbors. If there is neighbor, set distance and add
            // to queue.
            for ipp in &neigh[ip] {
                if imo[*ipp] > 0 || imo[*ipp] == IWSHED {
                    imd[ip] = 1;
                    fifo.push_back(ip as i32);
                    break;
                }
            }

            m += 1;
        }

        // Process the queue
        let mut ic_dist = 1;
        fifo.push_back(IFICT_PIXEL);

        while let Some(mut ip) = fifo.pop_front() {
            // Check for end of processing
            if ip == IFICT_PIXEL {
                if fifo.is_empty() {
                    break;
                } else {
                    fifo.push_back(IFICT_PIXEL);
                    ic_dist += 1;
                    ip = fifo.pop_front().unwrap();
                }
            }

            // Process queue
            for ipp in &neigh[ip as usize] {
                // Check for labeled watersheds or basins
                if imd[*ipp] < ic_dist && (imo[*ipp] > 0 || imo[*ipp] == IWSHED) {
                    if imo[*ipp] > 0 {
                        if imo[ip as usize] == MASK || imo[ip as usize] == IWSHED {
                            imo[ip as usize] = imo[*ipp];
                        } else if imo[ip as usize] != imo[*ipp] {
                            imo[ip as usize] = IWSHED;
                        }
                    } else if imo[ip as usize] == MASK {
                        imo[ip as usize] = IWSHED;
                    }
                } else if imo[*ipp] == MASK && imd[*ipp] == 0 {
                    imd[*ipp] = ic_dist + 1;
                    fifo.push_back(*ipp as i32);
                }
            }
        }

        // Check for mask values in IMO to identify new basins
        m = m_save;
        while m < count {
            let ip = ind[m];

            if imi[ip] != ih {
                break;
            }

            imd[ip] = 0;

            if imo[ip] == MASK {
                // ... New label for pixel
                ic_label += 1;
                fifo.push_back(ip as i32);
                imo[ip] = ic_label;

                // ... and all connected to it ...
                while let Some(ipp) = fifo.pop_front() {
                    for ippp in &neigh[ipp as usize] {
                        if imo[*ippp] == MASK {
                            fifo.push_back(*ippp as i32);
                            imo[*ippp] = ic_label;
                        }
                    }
                }
            }
            m += 1;
        }
    }

    // Find nearest neighbor of 0 watershed points and replace
    // use original input to check which group to affiliate with 0
    // Soring changes first in IMD to assure symetry in adjustment.
    for _ in 0..5 {
        imd = imo.clone();

        for jl in 0..count {
            let mut ipt = -1;
            if imo[jl] == 0 {
                let mut ep1 = max_value;

                for (ijn, jn) in neigh[jl].iter().enumerate() {
                    let diff = (data[jl] - data[*jn]).abs();
                    if diff <= ep1 && imo[*jn] != 0 {
                        ep1 = diff;
                        ipt = ijn as i32;
                    }
                }

                if ipt > 0 {
                    imd[jl] = imo[neigh[jl][ipt as usize]];
                }
            }
        }

        imo = imd.clone();
        let min_imo = imo.iter().min().unwrap_or(&-1);
        if *min_imo > 0 {
            break;
        }
    }

    Ok((imo, ic_label as usize + 1))
}

/// Implementation of:
/// Pierre Soille, Luc M. Vincent, "Determining watersheds in digital pictures via
/// flooding simulations", Proc. SPIE 1360, Visual Communications and Image Processing
/// '90: Fifth in a Series, (1 September 1990); doi: 10.1117/12.24211;
/// http://dx.doi.org/10.1117/12.24211
///
/// Adapted from https://github.com/mzur/watershed
///
pub fn watershed2(
    data: &[f64],
    width: usize,
    height: usize,
    steps: usize,
) -> Result<(Vec<i32>, usize), WatershedError> {
    const MASK: i32 = -2;
    const WSHD: i32 = 0;
    const INIT: i32 = -1;
    const INQE: i32 = -3;

    let size = width * height;
    if size != data.len() {
        return Err(WatershedError::InvalidData);
    }

    let mut current_label = 0;
    let mut flag = false;
    let mut fifo: VecDeque<usize> = VecDeque::new();
    let mut labels: Vec<i32> = vec![INIT; size];

    let neighbors = (0..size)
        .map(|i| nearest_neighbors(width, height, i))
        .collect::<Vec<_>>();

    let min_value = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_value = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Scale the data
    let fact = (steps as f64 - 1.0) / (max_value - min_value);

    // Digitize the signal, mapping each energy value to a level from 0 to steps
    let mut imi = data
        .iter()
        .map(|v| 1usize.max(steps.min((1.0 + (max_value - v) * fact).round() as usize)))
        .collect::<Vec<usize>>();

    let digital_min = imi.iter().min().unwrap().clone() as f64;
    let digital_max = imi.iter().max().unwrap().clone() as f64;

    let mut marker_partition = None;

    imi = imi
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if v >= &steps {
                marker_partition = Some(i);
                1
            } else {
                ((1.0 - ((digital_max - *v as f64) / (digital_max - digital_min))) * 255.0) as usize
            }
        })
        .collect();

    let indices = argsort(&imi);
    let sorted_data = indices
        .iter()
        .map(|i| *(&imi[*i].clone()))
        .collect::<Vec<usize>>();

    let levels = linspace(digital_min, digital_max, steps)
        .map(|v| v as usize)
        .collect::<Vec<usize>>();

    let mut level_indices: Vec<usize> = Vec::new();
    let mut current_level = 0;

    // Get the indices that delimit pixels with different values.
    for i in 0..size {
        if current_level < steps && sorted_data[i] > levels[current_level] {
            // Skip levels until the next highest one is reached.
            while current_level < steps && sorted_data[i] > levels[current_level] {
                current_level += 1;
            }
            level_indices.push(i);
        }
    }
    level_indices.push(size);

    let mut start_index = 0;

    for stop_index in level_indices {
        // Mask all pixels at the current level.
        for p in &indices[start_index..stop_index] {
            labels[*p] = MASK;

            // Initialize queue with neighbours of existing basins at the current level.
            for q in &neighbors[*p] {
                // p == q is ignored here because labels[p] < WSHD
                if labels[*q] >= WSHD {
                    labels[*p] = INQE;
                    fifo.push_back(*p);
                    break;
                }
            }
        }

        // Extend basins
        while let Some(p) = fifo.pop_front() {
            // Label i by inspecting neighbours.
            for q in &neighbors[p] {
                // Don't set lab_p in the outer loop because it may change.
                let label_p = labels[p];
                let label_q = labels[*q];

                if label_q > 0 {
                    if label_p == INQE || (label_p == WSHD && flag) {
                        labels[p] = label_q;
                    } else if label_p > 0 && label_p != label_q {
                        labels[p] = WSHD;
                        flag = false;
                    }
                } else if label_q == WSHD {
                    if label_p == INQE {
                        labels[p] = WSHD;
                        flag = true;
                    }
                } else if label_q == MASK {
                    labels[*q] = INQE;
                    fifo.push_back(*q);
                }
            }
        }

        // Detect and process new minima at the current level.
        for p in &indices[start_index..stop_index] {
            // i is inside a new minimum. Create a new label.
            if labels[*p] == MASK {
                current_label += 1;
                fifo.push_back(*p);
                labels[*p] = current_label;
                while let Some(q) = fifo.pop_front() {
                    for r in &neighbors[q] {
                        if labels[*r] == MASK {
                            fifo.push_back(*r);
                            labels[*r] = current_label;
                        }
                    }
                }
            }
        }

        // Increment
        start_index = stop_index;
    }

    // If there is a boundary layer, we count it the same as the watershed layer, it will count towards the
    // significant wave height but not toward the swell components
    if let Some(ip) = marker_partition {
        let nan_partition = labels[ip];
        labels.iter_mut().for_each(|v| {
            if *v == nan_partition {
                *v = 0;
            }
        });
    }

    Ok((labels, current_label as usize + 1))
}

#[cfg(test)]
mod tests {
    use super::lerp;
    use super::nearest_neighbors;
    use super::watershed;
    use rand;

    #[test]
    fn test_linear_interpolation() {
        assert!((lerp(&4.0, &5.0, &0.5) - 4.5) < 0.00001);
        assert!((lerp(&10.0, &15.0, &0.25) - 12.5) < 0.00001);
    }

    #[test]
    fn test_nearest_neighbors() {
        // Test given:
        // 0   1   2   3
        // 4   5   6   7
        // 8   9   10  11
        // 12  13  14  15
        //
        // Only wraps y and not x

        let i = 0;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 5);

        let i = 6;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 8);

        let i = 15;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 5);

        // 0   1   2   3  4  5
        // 5   7   8   9  10 11
        // 12  13  14  15 16 17
        // 18  19  20  21 22 23
        // 24  25  26  27 28 29
        let i = 5;
        let neighbors = nearest_neighbors(6, 5, i);
        assert_eq!(neighbors.len(), 5);
    }

    #[test]
    fn test_watershed_smoke() {
        const WIDTH: usize = 6;
        const HEIGHT: usize = 5;
        let data: [f64; WIDTH * HEIGHT] = rand::random();

        let watershed_result = watershed(&data, WIDTH, HEIGHT, 50, None);
        assert!(watershed_result.is_ok());
    }
}
