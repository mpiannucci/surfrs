use std::f64::consts::PI;

/// Calculates the scalar magnitude and heading angle in degrees from uv vector components
pub fn scalar_from_uv(u: f64, v: f64) -> (f64, i32) {
    let angle = (270.0 - (v.atan2(u) * (180.0 / PI))) as i32 % 360;
    let speed = (v.abs().powi(2) + u.abs().powi(2)).sqrt();
    (speed, angle)
}

#[cfg(test)]
mod tests {
    use crate::tools::math::scalar_from_uv;

	#[test]
	fn test_scaler_from_uv() {
        let u = -3.79485; 
        let v = 0.55966;
        let speed_control = 3.8359;
        // This direction is towards, we are computing from
        let direction_control = -81.61058f64.round() as i32 + 180;

        let (speed, direction) = scalar_from_uv(u, v);
        assert!((speed - speed_control).abs() < 0.0001); 
        assert_eq!(direction, direction_control); 
	}
}