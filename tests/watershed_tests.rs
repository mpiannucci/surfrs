#[path = "support/watershed.rs"]
mod support;

use serde::Deserialize;
use surfrs::tools::analysis::{nearest_neighbors, watershed};

#[derive(Deserialize)]
struct Snapshot {
    dataset: String,
    record: usize,
    labels: Vec<i32>,
    count: usize,
}

#[test]
fn partitions_match_original_forecast_and_buoy_maps() {
    // Full maps and counts captured before the performance changes. Preserve
    // label IDs, zero boundaries, blur behavior, and the returned count convention.
    let snapshots: Vec<Snapshot> =
        serde_json::from_str(include_str!("fixtures/watershed_partitions.json")).unwrap();
    let forecast = support::forecast_spectra();
    let buoy = support::buoy_spectra();
    assert_eq!(forecast.len(), 385);
    assert_eq!(buoy.len(), 1099);
    assert_eq!(snapshots.len(), 8);

    for snapshot in snapshots {
        let (spectra, blur) = match snapshot.dataset.as_str() {
            "forecast" => (&forecast, None),
            "buoy" => (&buoy, Some(0.8)),
            other => panic!("Unknown snapshot dataset: {other}"),
        };
        assert_eq!(
            spectra[snapshot.record].partition(100, blur).unwrap(),
            (snapshot.labels, snapshot.count),
            "{} record {}",
            snapshot.dataset,
            snapshot.record,
        );
    }
}

#[test]
fn neighbors_preserve_order_wrap_and_duplicates() {
    assert_eq!(nearest_neighbors(4, 4, 0), [1, 12, 4, 13, 5]);
    assert_eq!(nearest_neighbors(4, 4, 6), [5, 7, 2, 10, 2, 3, 9, 11]);
    assert_eq!(nearest_neighbors(4, 4, 15), [14, 11, 3, 11, 2]);
    assert_eq!(nearest_neighbors(1, 1, 0), [0, 0]);
    assert_eq!(nearest_neighbors(1, 3, 0), [2, 1]);
    assert_eq!(nearest_neighbors(3, 1, 1), [0, 2, 1, 1, 0, 2, 0, 2]);
}

#[test]
fn partitions_preserve_ties_and_boundary_cleanup() {
    // Quantized plateaus and several basins leave boundaries that exercise all
    // cleanup passes. These expected maps come from the original implementation.
    let energy = (0..30)
        .map(|i| ((i * 17 + i * i) % 11) as f64)
        .collect::<Vec<_>>();
    assert_eq!(
        watershed(&energy, 6, 5, 5, None).unwrap(),
        (
            vec![
                4, 1, 1, 0, 2, 2, 3, 4, 4, 4, 5, 5, 3, 3, 4, 4, 5, 5, 0, 1, 0, 1, 5, 5, 1, 1, 1, 5,
                5, 2
            ],
            6
        ),
    );
    assert_eq!(
        watershed(&energy, 6, 5, 5, Some(0.8)).unwrap(),
        (
            vec![
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1,
                1, 1
            ],
            3
        ),
    );
}

#[test]
fn partitions_preserve_flat_and_single_axis_grids() {
    for blur in [None, Some(0.8)] {
        for value in [0.0, 1.0] {
            assert_eq!(watershed(&[value], 1, 1, 100, blur).unwrap(), (vec![1], 2));
            assert_eq!(
                watershed(&[value; 30], 6, 5, 100, blur).unwrap(),
                (vec![1; 30], 2)
            );
        }
    }
    let energy = [0., 3., 2., 0., 1., 4., 0.];
    for (width, height) in [(1, 7), (7, 1)] {
        assert_eq!(
            watershed(&energy, width, height, 5, None).unwrap(),
            (vec![2, 2, 2, 1, 1, 1, 1], 3),
        );
        assert_eq!(
            watershed(&energy, width, height, 5, Some(0.8)).unwrap(),
            (vec![1, 1, 1, 1, 2, 2, 2], 3),
        );
    }
}
