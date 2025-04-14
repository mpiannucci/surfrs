/// Returns the sorted indices of the data vector in ascending order
pub fn argsort<T: Ord>(data: &[T]) -> Vec<usize> {
    let mut indices = (0..data.len()).collect::<Vec<_>>();
    indices.sort_by(|&a, &b| data[a].cmp(&data[b]));
    indices
}

/// Returns the sorted indices of the data vector in ascending order
pub fn argsort_partial<T: PartialOrd>(data: &[T]) -> Vec<usize> {
    let mut indices = (0..data.len()).collect::<Vec<_>>();
    indices.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap());
    indices
}

/// Returns the difference between the arrays n and n+1 items in a new vector
pub fn diff<T>(data: &[T]) -> Vec<T>
where
    T: std::ops::Sub<Output = T> + Copy,
{
    let last = data.len() - 1;

    (0..last + 1)
        .map(|i| match i {
            0 => data[1] - data[0],
            _ => data[i] - data[i - 1],
        })
        .collect()
}

/// Returns the difference between the arrays n and n+1 items in a new vector with absolute value applied
pub fn diff_abs(data: &[f64]) -> Vec<f64> {
    let last = data.len() - 1;

    (0..last + 1)
        .map(|i| match i {
            0 => (data[1] - data[0]).abs(),
            _ => (data[i] - data[i - 1]).abs(),
        })
        .collect()
}

/// Converts a float iterable to an u8 vector for simple packing
pub fn bin(data: &[f64], min: &f64, max: &f64, bin_count: &u8) -> Vec<u8> {
    data.iter()
        .map(|v| (((v - min) / (max - min)) * (*bin_count as f64)) as u8)
        .collect()
}

/// Converts a u8 iterable to an float vector from simple packing
pub fn unbin(data: &[u8], min: &f64, max: &f64, bin_count: &u8) -> Vec<f64> {
    data.iter()
        .map(|v| min + (((*v as f64) / (*bin_count as f64)) * (max - min)))
        .collect()
}

pub fn min_max(data: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    data.iter().for_each(|v| {
        if v.is_nan() {
            return;
        }

        if *v > max {
            max = *v;
        }

        if *v < min {
            min = *v;
        }
    });

    (min, max)
}

pub fn min_max_fill(data: &mut [f64], fill_value: f64) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    data.iter_mut().for_each(|v| {
        if v.is_nan() {
            *v = fill_value;
            return;
        }

        if *v > max {
            max = *v;
        }

        if *v < min {
            min = *v;
        }
    });

    (min, max)
}

#[cfg(test)]
mod tests {
    use crate::tools::vector::{min_max_fill, unbin};

    use super::{argsort, argsort_partial, bin, diff};

    #[test]
    fn test_argsort_int() {
        let test_data = vec![2, 1, 1, 3, 4, 5, 3, 8, 9, 4];
        let sorted_indexes_truth = vec![1, 2, 0, 3, 6, 4, 9, 5, 7, 8];

        let argsorted_indexes = argsort(&test_data);
        assert_eq!(argsorted_indexes.len(), sorted_indexes_truth.len());

        for i in 0..sorted_indexes_truth.len() {
            assert_eq!(sorted_indexes_truth[i], argsorted_indexes[i]);
        }
    }

    #[test]
    fn test_argsort_float() {
        let test_data = vec![1.0, 3.0, 0.5, 4.0, 9.0, 4.5, 5.4, 2.3, 8.9, 9.3, 6.7];
        let sorted_indexes_truth = vec![2, 0, 7, 1, 3, 5, 6, 10, 8, 4, 9];

        let argsorted_indexes = argsort_partial(&test_data);
        assert_eq!(argsorted_indexes.len(), sorted_indexes_truth.len());

        for i in 0..sorted_indexes_truth.len() {
            assert_eq!(sorted_indexes_truth[i], argsorted_indexes[i]);
        }
    }

    #[test]
    fn test_diff() {
        let test_data = vec![0, 2, 3, 4, 5, 8, 11, 15];
        let diff_truth = vec![2, 2, 1, 1, 1, 3, 3, 4];

        let result = diff(&test_data);
        assert_eq!(result.len(), test_data.len());

        for i in 0..diff_truth.len() {
            assert_eq!(diff_truth[i], result[i]);
        }
    }

    #[test]
    fn test_bin() {
        let test_data = vec![0.0, 0.0, 2.0, 3.0, 4.0, 5.0, 5.0, 5.0];
        let binned_result = vec![0u8, 0, 102, 153, 204, 255, 255, 255];

        let result = bin(&test_data, &-0.0, &5.0, &255);
        assert_eq!(result.len(), test_data.len());

        for i in 0..binned_result.len() {
            assert_eq!(binned_result[i], result[i]);
        }

        let unbinned_result = unbin(&result, &-0.0, &5.0, &255);
        for i in 0..unbinned_result.len() {
            assert!((unbinned_result[i] - test_data[i]).abs() < 0.00001);
        }
    }

    #[test]
    fn fix_vector() {
        let mut vector = vec![0., 1., 2., 3., 4., 5., f64::NAN];
        let (min, max) = min_max_fill(&mut vector, -9999.0);

        assert_eq!(min.floor() as i32, 0);
        assert_eq!(max.floor() as i32, 5);
        assert_eq!(vector[6].floor() as i32, -9999);
    }
}
