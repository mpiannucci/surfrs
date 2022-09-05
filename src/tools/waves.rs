use std::f64::consts::PI;

const GRAVITY: f64 = 9.81;

pub enum Error {
    ConvergenceFailure,
}

/// Computes the wavelength for a wave with the given period and depth. Units are metric, gravity is 9.81 m/s.
pub fn ldis(period: f64, depth: f64) -> Result<f64, Error> {
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

/// Calculate wavenumber and group velocity from the improved 
/// Eckard's formula by Beji (2003) using direct computation by approximation
/// 
/// Parameter list
/// ----------------------------------------------------------------
///  SI      Real   I   Intrinsic frequency (moving frame)  (rad/s)
///  H       Real   I   Waterdepth                            (m)
///  K       Real   O   Wavenumber                          (rad/m)
///  CG      Real   O   Group velocity                       (m/s)
/// ----------------------------------------------------------------
/// 
pub fn wavenu3(si: f64, h: f64) -> (f64, f64) {
    const ZPI: f64 = 2.0 * PI;
    const KDMAX: f64 = 20.0;

    let tp = si / ZPI;
    let kho = ZPI * ZPI * h / GRAVITY * tp * tp;
    let tmp = 1.55 + 1.3 * kho + 0.216 * kho * kho;
    let kh = kho * (1.0 + kho.powf(1.09) * 1.0 / tmp.min(KDMAX).exp()) / KDMAX.min(kho).tanh().sqrt();
    let k = kh / h;
    let cg = 0.5 * (1.0 + (2.0 * kh/KDMAX.min(2.0 * kh).sinh())) * si / k;

    (k, cg)
}

/// Solves for the Breaking Wave Height and Breaking Water Depth given a swell and beach conditions. All units are metric, degrees, and gravity is 9.81 m/s.
pub fn break_wave(
    period: f64,
    incident_angle: f64,
    deep_water_wave_height: f64,
    beach_slope: f64,
    water_depth: f64,
) -> Result<(f64, f64), Error> {
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

/// Calculate wavenumber and group velocity from the interpolation
/// array filled by DISTAB from a given intrinsic frequency and the
/// waterdepth.
// pub fn wav_nu()

/// Calculates the Compute mean parameters per swell component given a discretized specrtral signal
/// Ported from WW3 code: PTMEAN in w3partmd.f90
pub fn pt_mean(
    num_partitions: usize,
    partition_map: Vec<i32>,
    spectra: Vec<f64>,
    depth: f64,
    wind_speed: f64,
    wind_direction: f64,
    wave_numbers: Vec<f64>,
) -> () {

    ()
}














