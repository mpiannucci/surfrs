use std::f64::consts::PI;
use std::f64::{INFINITY, NEG_INFINITY};

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

pub enum Error {
    ConvergenceFailure,
}

/// Calculates the scalar magnitude and heading angle in degrees from uv vector components
pub fn scalar_from_uv(u: f64, v: f64) -> (f64, f64) {
    let angle = (270.0 - (v.atan2(u) * (180.0 / PI))) as i32 % 360;
    let speed = (v.abs().powi(2) + u.abs().powi(2)).sqrt();
    (speed, angle as f64)
}

/// Computes the wavelength for a wave with the given period and depth. Units are metric, gravity is 9.81 m/s.
pub fn ldis(period: f64, depth: f64) -> Result<f64, Error> {
    const GRAVITY: f64 = 9.81;
    const EPS: f64 = 0.000001;
    const MAX_ITERATION: usize = 50;

    let omega = 2.0 * PI / period;
    let d = omega.powi(2) * depth * GRAVITY;

    let mut iter: usize = 0;
    let mut err: f64 = 1.0;

    let mut xf: f64 = 0.0;
    let mut xo: f64;
    let mut f: f64;
    let mut df: f64;

    // Make an initial guess for non dimensional solutions
    if d >= 1.0 {
        xo = d;
    } else {
        xo = d.sqrt();
    }

    // Solve using newton raphson iteration
    while (err > EPS) && (iter < MAX_ITERATION) {
        f = xo - (d / xo.tanh());
        df = 1.0 + (d / xo.sinh().powi(2));
        xf = xo - (f / df);
        err = ((xf - xo) / xo).abs();
        xo = xf;
        iter += 1;
    }

    if iter >= MAX_ITERATION {
        Err(Error::ConvergenceFailure)
    } else {
        Ok(2.0 * PI * depth / xf)
    }
}

/// Solves for the Breaking Wave Height and Breaking Water Depth given a swell and beach conditions. All units are metric, degrees, and gravity is 9.81 m/s.
pub fn break_wave(
    period: f64,
    incident_angle: f64,
    deep_water_wave_height: f64,
    beach_slope: f64,
    water_depth: f64,
) -> Result<(f64, f64), Error> {
    const GRAVITY: f64 = 9.81;

    // We need the angle in radians
    let incident_angle = incident_angle.to_radians();

    // Propagate the error up if ldis fails
    let wavelength = ldis(period, water_depth)?;

    let deep_wavelength = (GRAVITY * period.powi(2)) / (2.0 * PI);
    let initial_celerity = (GRAVITY * period) / (2.0 * PI);
    let celerity = wavelength / period;
    let theta = (celerity * (incident_angle.sin() / initial_celerity)).asin();
    let refraction_coefficient = (incident_angle.cos() / theta.cos()).sqrt();
    let a = 43.8 * (1.0 - (-19.0 * beach_slope).exp());
    let b = 1.56 / (1.0 + (-19.5 * beach_slope).exp());
    let deep_refracted_wave_height = refraction_coefficient * deep_water_wave_height;
    let w = 0.56 * (deep_refracted_wave_height / deep_wavelength).powf(-0.2);

    // Finally calculate the breaking wave height
    let breaking_wave_height = w * deep_refracted_wave_height;

    // And the breaking wave depth
    let k = b - a * (breaking_wave_height / (GRAVITY * period.powi(2)));
    let breaking_water_depth = breaking_wave_height / k;

    Ok((breaking_wave_height, breaking_water_depth))
}

/// Calculate the refraction coefficient Kr with given inputs on a straight beach with parrellel bottom contours.
/// Assumes angles in degrees and metric units
/// Returns the refraction coefficient and the shallow incident angle in degrees
pub fn refraction_coefficient(wavelength: f64, depth: f64, incident_angle: f64) -> (f64, f64) {
    let incident_angle = incident_angle.to_radians();
    let wavenumber = (2.0 * PI) / wavelength;
    let shallow_incident_angle = (incident_angle.sin() * (wavenumber * depth).tanh()).asin();
    let refraction_coeff = (incident_angle.cos() / shallow_incident_angle.cos()).sqrt();
    let shallow_incident_angle = shallow_incident_angle.to_degrees();
    (refraction_coeff, shallow_incident_angle)
}

/// Calculate the shoaling coeffecient Ks. Units are metric, gravity is 9.81
pub fn shoaling_coefficient(wavelength: f64, depth: f64) -> f64 {
    const GRAVITY: f64 = 9.81;

    // Solve basic dispersion relationships
    let wavenumber = (2.0 * PI) / wavelength;
    let deep_wavelength = wavelength / (wavenumber * depth).tanh();
    let w = (wavenumber * GRAVITY).sqrt();
    let period = (2.0 * PI) / w;

    // Solve celerity
    let initial_celerity = deep_wavelength / period;
    let celerity = initial_celerity * (wavenumber * depth).tanh();
    let group_velocity =
        0.5 * celerity * (1.0 + ((2.0 * wavenumber * depth) / (2.0 * wavenumber * depth).sinh()));
    (initial_celerity / (2.0 * group_velocity)).sqrt()
}

/// Calculates the zero moment of a wave spectra point given energy and bandwidth
pub fn zero_spectral_moment(energy: f64, bandwidth: f64) -> f64 {
    energy * bandwidth
}

/// Calculates the second moment of a wave spectra point given enrgy, frequency and bandwith
pub fn second_spectral_moment(energy: f64, bandwidth: f64, frequency: f64) -> f64 {
    energy * bandwidth * frequency.powi(2)
}

/// Calculates the steepness coefficient given the wave spectra moments. Assumes metric with gravity 9.81 m/s
pub fn steepness_coefficient(zero_moment: f64, second_moment: f64) -> f64 {
    (8.0 * PI * second_moment) / (9.81 * zero_moment.sqrt())
}

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

pub fn closest_model_datetime(datetime: DateTime<Utc>) -> DateTime<Utc> {
    let adjusted = datetime + Duration::hours(-6);
    let latest_model_hour = adjusted.hour() % 6;
    (adjusted - Duration::hours(latest_model_hour as i64))
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
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

