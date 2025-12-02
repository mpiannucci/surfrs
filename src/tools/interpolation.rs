/// PCHIP (Piecewise Cubic Hermite Interpolating Polynomial) interpolation
///
/// This module provides monotonicity-preserving cubic interpolation using the
/// Fritsch-Carlson algorithm. Unlike standard cubic splines, PCHIP avoids
/// overshoots and preserves the shape of the data.

/// PCHIP interpolator for 1D data
///
/// Uses the Fritsch-Carlson algorithm for monotonic cubic interpolation.
/// This preserves monotonicity and avoids overshoots common in standard cubic splines.
pub struct PchipInterpolator {
    x: Vec<f64>,
    y: Vec<f64>,
    slopes: Vec<f64>,
}

impl PchipInterpolator {
    /// Create a new PCHIP interpolator from sorted (x, y) data.
    ///
    /// # Arguments
    /// * `x` - Strictly increasing x-coordinates (must be sorted ascending)
    /// * `y` - Corresponding y-values
    ///
    /// # Panics
    /// Panics if x and y have different lengths or if length < 2
    pub fn new(x: &[f64], y: &[f64]) -> Self {
        assert_eq!(x.len(), y.len(), "x and y must have the same length");
        assert!(x.len() >= 2, "Need at least 2 points for interpolation");

        let n = x.len();
        let mut slopes = vec![0.0; n];

        // Handle special case of only 2 points
        if n == 2 {
            let slope = (y[1] - y[0]) / (x[1] - x[0]);
            slopes[0] = slope;
            slopes[1] = slope;
            return PchipInterpolator {
                x: x.to_vec(),
                y: y.to_vec(),
                slopes,
            };
        }

        // Compute secants (finite differences)
        let mut h: Vec<f64> = Vec::with_capacity(n - 1);
        let mut delta: Vec<f64> = Vec::with_capacity(n - 1);

        for i in 0..n - 1 {
            h.push(x[i + 1] - x[i]);
            delta.push((y[i + 1] - y[i]) / h[i]);
        }

        // Compute slopes using Fritsch-Carlson method
        // Boundary slopes use one-sided differences
        slopes[0] = delta[0];
        slopes[n - 1] = delta[n - 2];

        // Interior slopes
        for i in 1..n - 1 {
            if delta[i - 1].signum() != delta[i].signum() || delta[i - 1] == 0.0 || delta[i] == 0.0 {
                // Different signs or zero - set slope to zero for monotonicity
                slopes[i] = 0.0;
            } else {
                // Weighted harmonic mean of adjacent secants
                let w1 = 2.0 * h[i] + h[i - 1];
                let w2 = h[i] + 2.0 * h[i - 1];
                slopes[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
            }
        }

        // Apply monotonicity constraints (Fritsch-Carlson conditions)
        for i in 0..n - 1 {
            if delta[i] == 0.0 {
                // Flat region - zero slopes at both ends
                slopes[i] = 0.0;
                slopes[i + 1] = 0.0;
            } else {
                let alpha = slopes[i] / delta[i];
                let beta = slopes[i + 1] / delta[i];

                // Check if we need to rescale to maintain monotonicity
                // Condition: alpha^2 + beta^2 <= 9
                let tau = alpha * alpha + beta * beta;
                if tau > 9.0 {
                    let tau_sqrt = tau.sqrt();
                    slopes[i] = 3.0 * delta[i] * alpha / tau_sqrt;
                    slopes[i + 1] = 3.0 * delta[i] * beta / tau_sqrt;
                }
            }
        }

        PchipInterpolator {
            x: x.to_vec(),
            y: y.to_vec(),
            slopes,
        }
    }

    /// Interpolate at a single point.
    ///
    /// For points outside the data range, returns the boundary value (clamped).
    pub fn interpolate(&self, x_new: f64) -> f64 {
        let n = self.x.len();

        // Handle extrapolation by clamping to boundary values
        if x_new <= self.x[0] {
            return self.y[0];
        }
        if x_new >= self.x[n - 1] {
            return self.y[n - 1];
        }

        // Binary search for the interval containing x_new
        let k = self.find_interval(x_new);

        // Hermite cubic interpolation
        let h = self.x[k + 1] - self.x[k];
        let t = (x_new - self.x[k]) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        h00 * self.y[k]
            + h10 * h * self.slopes[k]
            + h01 * self.y[k + 1]
            + h11 * h * self.slopes[k + 1]
    }

    /// Interpolate at multiple points.
    pub fn interpolate_many(&self, x_new: &[f64]) -> Vec<f64> {
        x_new.iter().map(|&x| self.interpolate(x)).collect()
    }

    /// Find the interval [x[k], x[k+1]] containing the given x value using binary search.
    fn find_interval(&self, x: f64) -> usize {
        let mut lo = 0;
        let mut hi = self.x.len() - 1;

        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if x < self.x[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }
}

/// Interpolate with circular (periodic) boundary conditions.
///
/// This is useful for direction interpolation where 0° and 360° are the same.
///
/// # Arguments
/// * `source_dir` - Source direction bins in degrees (must be sorted ascending, 0-360 range)
/// * `values` - Values at each source direction
/// * `target_dir` - Target direction in degrees (0-360 range)
///
/// # Returns
/// Interpolated value at target_dir using circular PCHIP interpolation
pub fn circular_pchip_interpolate(
    source_dir: &[f64],
    values: &[f64],
    target_dir: f64,
) -> f64 {
    assert_eq!(source_dir.len(), values.len());
    let n = source_dir.len();

    // Create wrapped arrays for periodic boundary: [dir-360, dir, dir+360]
    let mut wrapped_dir = Vec::with_capacity(3 * n);
    let mut wrapped_val = Vec::with_capacity(3 * n);

    // dir - 360
    for &d in source_dir {
        wrapped_dir.push(d - 360.0);
    }
    // dir (original)
    for &d in source_dir {
        wrapped_dir.push(d);
    }
    // dir + 360
    for &d in source_dir {
        wrapped_dir.push(d + 360.0);
    }

    // Replicate values three times
    for &v in values {
        wrapped_val.push(v);
    }
    for &v in values {
        wrapped_val.push(v);
    }
    for &v in values {
        wrapped_val.push(v);
    }

    let interp = PchipInterpolator::new(&wrapped_dir, &wrapped_val);
    interp.interpolate(target_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pchip_linear_data() {
        // Linear data should interpolate exactly
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];
        let pchip = PchipInterpolator::new(&x, &y);

        assert!((pchip.interpolate(0.5) - 0.5).abs() < 1e-10);
        assert!((pchip.interpolate(1.5) - 1.5).abs() < 1e-10);
        assert!((pchip.interpolate(2.5) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_pchip_exact_at_knots() {
        // Should pass through all data points exactly
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 0.5, 0.3, 0.8, 1.0];
        let pchip = PchipInterpolator::new(&x, &y);

        for i in 0..x.len() {
            assert!((pchip.interpolate(x[i]) - y[i]).abs() < 1e-10,
                "Failed at knot {}: expected {}, got {}", i, y[i], pchip.interpolate(x[i]));
        }
    }

    #[test]
    fn test_pchip_monotonic_increasing() {
        // Monotonically increasing data should stay monotonic
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 0.1, 0.5, 0.9, 1.0];
        let pchip = PchipInterpolator::new(&x, &y);

        let mut prev = pchip.interpolate(0.0);
        for i in 1..40 {
            let x_val = i as f64 * 0.1;
            let curr = pchip.interpolate(x_val);
            assert!(curr >= prev - 1e-10,
                "Monotonicity violated at x={}: prev={}, curr={}", x_val, prev, curr);
            prev = curr;
        }
    }

    #[test]
    fn test_pchip_no_overshoot() {
        // Step-like data should not overshoot
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.0, 0.0, 1.0, 1.0];
        let pchip = PchipInterpolator::new(&x, &y);

        for i in 0..30 {
            let x_val = i as f64 * 0.1;
            let val = pchip.interpolate(x_val);
            assert!(val >= -1e-10 && val <= 1.0 + 1e-10,
                "Overshoot at x={}: y={}", x_val, val);
        }
    }

    #[test]
    fn test_pchip_boundary_clamping() {
        // Extrapolation should clamp to boundary values
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 30.0];
        let pchip = PchipInterpolator::new(&x, &y);

        assert!((pchip.interpolate(0.0) - 10.0).abs() < 1e-10);  // Below range
        assert!((pchip.interpolate(5.0) - 30.0).abs() < 1e-10);  // Above range
    }

    #[test]
    fn test_pchip_two_points() {
        // Should handle 2-point case (linear interpolation)
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let pchip = PchipInterpolator::new(&x, &y);

        assert!((pchip.interpolate(0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_circular_interpolation_basic() {
        // Basic circular interpolation
        let dir = vec![0.0, 90.0, 180.0, 270.0];
        let vals = vec![1.0, 2.0, 1.0, 2.0];

        // At 45 degrees, should be between 1.0 and 2.0
        let result = circular_pchip_interpolate(&dir, &vals, 45.0);
        assert!(result > 1.0 && result < 2.0, "Expected value between 1 and 2, got {}", result);
    }

    #[test]
    fn test_circular_interpolation_wrap() {
        // Test wrap-around at 0/360 boundary
        let dir = vec![0.0, 90.0, 180.0, 270.0];
        let vals = vec![1.0, 2.0, 3.0, 2.0];

        // At 315 degrees (between 270 and 360/0)
        let result = circular_pchip_interpolate(&dir, &vals, 315.0);
        // Should be between the values at 270 (2.0) and 0 (1.0)
        assert!(result >= 1.0 && result <= 2.0,
            "Expected value between 1 and 2 at 315 deg, got {}", result);

        // At 350 degrees (close to 0/360)
        let result2 = circular_pchip_interpolate(&dir, &vals, 350.0);
        assert!(result2 >= 1.0 && result2 <= 2.0,
            "Expected value between 1 and 2 at 350 deg, got {}", result2);
    }

    #[test]
    fn test_circular_exact_at_knots() {
        // Should pass through data points
        let dir = vec![0.0, 90.0, 180.0, 270.0];
        let vals = vec![1.0, 2.0, 3.0, 4.0];

        for i in 0..dir.len() {
            let result = circular_pchip_interpolate(&dir, &vals, dir[i]);
            assert!((result - vals[i]).abs() < 1e-10,
                "Expected {} at {} deg, got {}", vals[i], dir[i], result);
        }
    }
}
