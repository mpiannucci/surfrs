use std::{
    collections::VecDeque,
    f64::{INFINITY, NEG_INFINITY},
};

use crate::tools::{linspace::linspace, vector::argsort_float};

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

    let mut min_val = INFINITY;
    let mut max_val = NEG_INFINITY;
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
/// https://stackoverflow.com/questions/7862190/nearest-neighbor-operation-on-1d-array-elements
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
/// 1   2   3   
/// 5   6   7   
/// 9   10  11
///
/// this implementation does NOT wrap, and defaults to the selected index in cases where wrapping would
/// occur
///
pub fn nearest_neighbors(width: usize, height: usize, index: usize) -> [usize; 9] {
    let left = if index == 0 { index } else { index - 1 };

    let right = if index == (width * height - 1) {
        index
    } else {
        index + 1
    };

    let top = if index < width { index } else { index - width };

    let bottom = if index >= ((width * height) - width) {
        index
    } else {
        index + width
    };

    let top_left = if top % width == 0 { top } else { top - 1 };

    let top_right = if top == (width * height - 1) {
        top
    } else {
        top + 1
    };

    let bottom_left = if bottom % width == 0 {
        bottom
    } else {
        bottom - 1
    };

    let bottom_right = if bottom == (width * height - 1) {
        bottom
    } else {
        bottom + 1
    };

    return [
        top_left,
        top,
        top_right,
        left,
        index,
        right,
        bottom_left,
        bottom,
        bottom_right,
    ];
}

#[derive(Debug)]
pub enum WatershedError {
    Unknown,
    InvalidData,
}

/// Implementation of:
/// Pierre Soille, Luc M. Vincent, "Determining watersheds in digital pictures via
/// flooding simulations", Proc. SPIE 1360, Visual Communications and Image Processing
/// '90: Fifth in a Series, (1 September 1990); doi: 10.1117/12.24211;
/// http://dx.doi.org/10.1117/12.24211
///
/// Adapted from https://github.com/mzur/watershed
///
pub fn watershed(
    data: &[f64],
    width: usize,
    height: usize,
    steps: u8,
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

    let indices = argsort_float(&data);
    let sorted_data = indices
        .iter()
        .map(|i| *(&data[*i].clone()))
        .collect::<Vec<f64>>();

    let min_value = sorted_data[0];
    let max_value = sorted_data[sorted_data.len() - 1];
    let range = max_value - min_value;
    let factor = (steps as f64 - 1.0) / range;
    let binned_data: Vec<u8> = sorted_data.iter().map(|s| 0.max(steps.min((1.0 + (factor * (max_value - s))).round() as u8))).collect();
    println!("max: {max_value}, min: {min_value}");
    println!("{:?}", data);
    println!("{:?}", binned_data);

    let mut level_indices: Vec<usize> = Vec::new();
    let mut current_level = 0;

    // Get the indices that deleimit pixels with different values.
    for i in 0..size {
        if binned_data[i] > current_level {
            // Skip levels until the next highest one is reached.
            while binned_data[i] > current_level {
                current_level += 1;
            }
            level_indices.push(i);
        }
    }
    level_indices.push(size);

    let mut start_index = 0;

    for stop_index in level_indices {
        // Mask all pixels at the current level.
        for si in &indices[start_index..stop_index] {
            labels[*si] = MASK;

            // Initialize queue with neighbours of existing basins at the current level.
            for n in neighbors[*si] {
                // p == q is ignored here because labels[p] < WSHD
                if labels[n] >= WSHD {
                    labels[*si] = INQE;
                    fifo.push_back(*si);
                    break;
                }
            }
        }

        // Extend basins
        while let Some(i) = fifo.pop_front() {
            // Label i by inspecting neighbours.
            for n in neighbors[i] {
                // Don't set lab_p in the outer loop because it may change.
                let label_i = labels[i];
                let label_n = labels[n];

                if label_n > 0 {
                    if label_i == INQE || (label_i == WSHD && flag) {
                        labels[i] = label_n;
                    } else if label_i > 0 && label_i != label_n {
                        labels[i] = WSHD;
                        flag = false;
                    }
                } else if label_n == WSHD {
                    if label_i == INQE {
                        labels[i] = WSHD;
                        flag = true;
                    }
                } else if label_n == MASK {
                    labels[n] = INQE;
                    fifo.push_back(n);
                }
            }
        }

        // Detect and process new minima at the current level.
        for i in &indices[start_index..stop_index] {
            // i is inside a new minimum. Create a new label.
            if labels[*i] == MASK {
                current_label += 1;
                fifo.push_back(*i);
                labels[*i] = current_label;
                while let Some(ii) = fifo.pop_front() {
                    for n in neighbors[ii] {
                        if labels[n] == MASK {
                            fifo.push_back(n);
                            labels[n] = current_label;
                        }
                    }
                }
            }
        }

        // Increment
        start_index = stop_index;
    }

    Ok((labels, current_label as usize))
}

#[cfg(test)]
mod tests {
    use crate::tools::analysis::watershed;

    use super::nearest_neighbors;
    use rand;

    #[test]
    fn test_nearest_neighbors() {
        let i = 0;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors[0], 0);
        assert_eq!(neighbors[1], 0);
        assert_eq!(neighbors[2], 1);
        assert_eq!(neighbors[3], 0);
        assert_eq!(neighbors[4], 0);
        assert_eq!(neighbors[5], 1);
        assert_eq!(neighbors[6], 4);
        assert_eq!(neighbors[7], 4);
        assert_eq!(neighbors[8], 5);

        let i = 6;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors[0], 1);
        assert_eq!(neighbors[1], 2);
        assert_eq!(neighbors[2], 3);
        assert_eq!(neighbors[3], 5);
        assert_eq!(neighbors[4], 6);
        assert_eq!(neighbors[5], 7);
        assert_eq!(neighbors[6], 9);
        assert_eq!(neighbors[7], 10);
        assert_eq!(neighbors[8], 11);

        let i = 15;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors[0], 10);
        assert_eq!(neighbors[1], 11);
        assert_eq!(neighbors[2], 12);
        assert_eq!(neighbors[3], 14);
        assert_eq!(neighbors[4], 15);
        assert_eq!(neighbors[5], 15);
        assert_eq!(neighbors[6], 14);
        assert_eq!(neighbors[7], 15);
        assert_eq!(neighbors[8], 15);
    }

    #[test]
    fn test_watershed_smoke() {
        const WIDTH: usize = 6;
        const HEIGHT: usize = 5;
        let data: [f64; WIDTH * HEIGHT] = rand::random();

        let watershed_result = watershed(&data, WIDTH, HEIGHT, 100);
        assert!(watershed_result.is_ok());
    }
}
