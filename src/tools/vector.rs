
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
pub fn diff<T>(data: &[T]) -> Vec<T> where T: std::ops::Sub<Output=T> + Copy {
	let last = data.len() - 1;

	(0..last + 1)
		.map(|i| {
			match i {
				0 => data[1] - data[0],
				_ => data[i] - data[i - 1],
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
    use super::{argsort, argsort_partial, diff};

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
}
