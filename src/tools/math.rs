use std::f64::consts::PI;

/// Calculates the scalar magnitude and heading angle in degrees from uv vector components
pub fn scalar_from_uv(u: f64, v: f64) -> (f64, i32) {
    let angle = (270.0 - (v.atan2(u) * (180.0 / PI))) as i32 % 360;
    let speed = (v.abs().powi(2) + u.abs().powi(2)).sqrt();
    (speed, angle)
}

pub fn f_eq(v: f64, val: f64) -> bool {
    (v - val).abs() < 0.00001
}

pub fn is_some_missing(v: f64, missing: f64) -> Option<f64> {
    if f_eq(v, missing) {
        None
    } else {
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::{scalar_from_uv, is_some_missing};

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

    #[test]
    fn test_missing_value() {
        const MISSING: f64 = 999.0;
        let v1 = 30.0;
        let v2 = 999.0;

        assert!(is_some_missing(v1, MISSING).is_some());
        assert!(is_some_missing(v2, MISSING).is_none());
    }
}