
/// Returns the sorted indices of the data vector in ascending order
pub fn argsort_float(data: &[f64]) -> Vec<usize> {
    let mut indices = (0..data.len()).collect::<Vec<_>>();
    indices.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap());
    indices
}

#[cfg(test)]
mod tests {
    use super::argsort_float;


	#[test]
	fn test_argsort_float() {
		let test_data = vec![1.0, 3.0, 0.5, 4.0, 9.0, 4.5, 5.4, 2.3, 8.9, 9.3, 6.7];
		let sorted_indexes_truth = vec![2, 0, 7, 1, 3, 5, 6, 10, 8, 4, 9];

		let argsorted_indexes = argsort_float(&test_data);
		assert_eq!(argsorted_indexes.len(), sorted_indexes_truth.len());

		for i in 0..sorted_indexes_truth.len() {
			assert_eq!(sorted_indexes_truth[i], argsorted_indexes[i]);
		}
	}
}
