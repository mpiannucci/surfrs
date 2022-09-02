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
       neighbors.push(t - (width - 1 - i)); 
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
    if i != width - 1 && j == height -1 {
        neighbors.push(index + 1 - width * (height - 1));
    }

    neighbors
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
// pub fn watershed(
//     data: &[f64],
//     width: usize,
//     height: usize,
//     steps: u8,
// ) -> Result<(Vec<i32>, usize), WatershedError> {
//     const MASK: i32 = -2;
//     const WSHD: i32 = 0;
//     const INIT: i32 = -1;
//     const INQE: i32 = -3;

//     let size = width * height;
//     if size != data.len() {
//         return Err(WatershedError::InvalidData);
//     }

//     let mut current_label = 0;
//     let mut flag = false;
//     let mut fifo: VecDeque<usize> = VecDeque::new();
//     let mut labels: Vec<i32> = vec![INIT; size];

//     let neighbors = (0..size)
//         .map(|i| nearest_neighbors(width, height, i))
//         .collect::<Vec<_>>();

//     let indices = argsort_float(&data);
//     let sorted_data = indices
//         .iter()
//         .map(|i| *(&data[*i].clone()))
//         .collect::<Vec<f64>>();

//     let min_value = sorted_data[0];
//     let max_value = sorted_data[sorted_data.len() - 1];
//     let range = max_value - min_value;
//     let factor = (steps as f64 - 1.0) / range;
//     let binned_data: Vec<u8> = sorted_data.iter().map(|s| 0.max(steps.min((1.0 + (factor * (max_value - s))).round() as u8))).rev().collect();
//     // println!("max: {max_value}, min: {min_value}");
//     // println!("{:?}", data);
//     // println!("{:?}", binned_data);

//     let mut level_indices: Vec<usize> = Vec::new();
//     let mut current_level = 0;

//     // Get the indices that deleimit pixels with different values.
//     for i in 0..size {
//         if binned_data[i] > current_level {
//             // println!("{}", binned_data[i]);
//             // Skip levels until the next highest one is reached.
//             while binned_data[i] > current_level {
//                 current_level += 1;
//             }
//             // println!("{current_level}");
//             level_indices.push(i);
//         }
//     }
//     level_indices.push(size);
//     // println!("{:?}", level_indices);

//     let mut start_index = 0;

//     for stop_index in level_indices {
//         // Mask all pixels at the current level.
//         for si in &indices[start_index..stop_index] {
//             labels[*si] = MASK;

//             // Initialize queue with neighbours of existing basins at the current level.
//             for n in neighbors[*si] {
//                 // p == q is ignored here because labels[p] < WSHD
//                 if labels[n] >= WSHD {
//                     labels[*si] = INQE;
//                     fifo.push_back(*si);
//                     break;
//                 }
//             }
//         }

//         // Extend basins
//         while let Some(i) = fifo.pop_front() {
//             // Label i by inspecting neighbours.
//             for n in neighbors[i] {
//                 // Don't set lab_p in the outer loop because it may change.
//                 let label_i = labels[i];
//                 let label_n = labels[n];

//                 if label_n > 0 {
//                     if label_i == INQE || (label_i == WSHD && flag) {
//                         labels[i] = label_n;
//                     } else if label_i > 0 && label_i != label_n {
//                         labels[i] = WSHD;
//                         flag = false;
//                     }
//                 } else if label_n == WSHD {
//                     if label_i == INQE {
//                         labels[i] = WSHD;
//                         flag = true;
//                     }
//                 } else if label_n == MASK {
//                     labels[n] = INQE;
//                     fifo.push_back(n);
//                 }
//             }
//         }

//         // Detect and process new minima at the current level.
//         for i in &indices[start_index..stop_index] {
//             // i is inside a new minimum. Create a new label.
//             if labels[*i] == MASK {
//                 current_label += 1;
//                 fifo.push_back(*i);
//                 labels[*i] = current_label;
//                 while let Some(ii) = fifo.pop_front() {
//                     for n in neighbors[ii] {
//                         if labels[n] == MASK {
//                             fifo.push_back(n);
//                             labels[n] = current_label;
//                         }
//                     }
//                 }
//             }
//         }

//         // Increment
//         start_index = stop_index;
//     }

//     println!("{:?}", labels);

//     Ok((labels, current_label as usize))
// }

#[cfg(test)]
mod tests {
    use super::nearest_neighbors;
    use rand;

    #[test]
    fn test_nearest_neighbors() {
        // Test given: 
        // 0   1   2   3
        // 4   5   6   7
        // 8   9   10  11
        // 12  13  14  15
        //
        // Only wraps rows and not columns

        let i = 0;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 5);

        let i = 6;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 8);

        let i = 15;
        let neighbors = nearest_neighbors(4, 4, i);
        assert_eq!(neighbors.len(), 5);
    }

    // #[test]
    // fn test_watershed_smoke() {
    //     const WIDTH: usize = 6;
    //     const HEIGHT: usize = 5;
    //     let data: [f64; WIDTH * HEIGHT] = rand::random();

    //     let watershed_result = watershed(&data, WIDTH, HEIGHT, 100);
    //     assert!(watershed_result.is_ok());
    // }
}
