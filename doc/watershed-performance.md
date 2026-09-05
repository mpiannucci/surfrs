# Watershed performance

The watershed hot path stores each cell's neighbor indices inline, groups its
quantized levels with a stable counting sort, and reuses the two boundary-cleanup
buffers. Public signatures, neighbor order and duplicates, level tie order,
quantization, blur, and simultaneous boundary updates are preserved.

The largest improvement comes from removing the allocation and growth of a
separate neighbor vector for every spectral cell. Counting sort avoids comparison
sorting of values that are already `u8`. Cleanup copies into the existing scratch
buffer and swaps buffers instead of allocating two clones on each pass.

## Measurements

Measured on Apple M2 with rustc 1.98.0 and image 0.24.9, using Cargo release
defaults. A separate comparison harness ran the implementation from commit
`548473d68beed10b5b26cff29ce3dd694527c10e` and the updated implementation in the
same executable, rotating variant order between rounds:

| Input | Original | Updated | Speedup |
|---|---:|---:|---:|
| Forecast, 50 × 36, no blur | 204.24 µs | 83.62 µs | 2.44× |
| Buoy, 64 × 36, blur 0.8 | 301.21 µs | 144.83 µs | 2.08× |

These are medians of eight batch averages after warm-up. Each batch performs 16
passes over 32 evenly spaced spectra from the repository fixtures. Timing includes
the complete watershed call, blur when configured, and disposal of returned
labels. Fixture parsing and buoy reconstruction are outside the timed region.
These are function-level improvements, not application-level speedups.

The repository includes a benchmark of the current public `Spectra::partition`
path with the same inputs and sampling:

```sh
cargo bench --bench watershed
```

Its measured times were 84.63 µs for forecast and 152.23 µs for buoy. Absolute
times vary with build context and machine load; compare revisions under the same
conditions. The benchmark requires no network requests or new dependencies.

## Compatibility checks

The updated implementation matched complete label vectors and returned counts
against the original for 3,020 cases: all 385 forecast and 1,099 buoy fixture
records, plus 1,536 synthetic cases spanning ties, flat spectra, small and
single-axis grids, several level counts, nonfinite inputs, and blur/no-blur.

The existing unit tests in `src/tools/analysis.rs` now check exact neighbor
lists and one small partition map with and without blur. The expected labels
were captured from the original implementation and cover level ties and boundary
cleanup. The existing integration tests continue to exercise real spectra.

```sh
cargo test --lib
cargo test --test buoy_data_tests read_wave_spectra_data
cargo test --test buoy_data_tests read_spectral_forecast_station_data
```

Neighbor topology and other scratch buffers could also be retained across calls
through a caller-owned workspace. That requires a separate API/lifecycle design
and is not part of this change. The original port's duplicate interior bottom
neighbor and its exclusion of neighbor-list index zero during boundary selection
are preserved; correctness changes to those rules should be evaluated separately.
