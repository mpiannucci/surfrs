use std::f64::consts::PI;

/// Calculates the scalar magnitude and heading angle in degrees from uv vector components
pub fn scalar_from_uv(u: f64, v: f64) -> (f64, f64) {
    let angle = (270.0 - (v.atan2(u) * (180.0 / PI))) as i32 % 360;
    let speed = (v.abs().powi(2) + u.abs().powi(2)).sqrt();
    (speed, angle as f64)
}

#[cfg(test)]
mod tests {
    use crate::tools::math::scalar_from_uv;

	#[test]
	fn test_scaler_from_uv() {
		// TODO
	}
}