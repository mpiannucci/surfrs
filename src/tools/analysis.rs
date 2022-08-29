use std::f64::{INFINITY, NEG_INFINITY};

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
    let left = if index == 0 {
        index
    } else {
        index - 1
    };

    let right = if index == (width * height - 1) {
        index
    } else {
        index + 1
    };

    let top = if index < width {
        index
    } else {
        index - width
    };

    let bottom = if index > ((width*height) - width) {
        index
    } else {
        index + width
    };

    let top_left = if top % width == 0 {
        top
    } else {
        top - 1
    };

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
        top_left, top, top_right,
        left, index, right,
        bottom_left, bottom, bottom_right,
    ];
}

#[cfg(test)]
mod tests {
    use super::nearest_neighbors;

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
}