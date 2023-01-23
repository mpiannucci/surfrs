use std::{f64::consts::PI, ops::Sub, vec};

use crate::{swell::Swell, units::{Direction, direction::DirectionConvention, UnitSystem}};

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
    let kh =
        kho * (1.0 + kho.powf(1.09) * 1.0 / tmp.min(KDMAX).exp()) / KDMAX.min(kho).tanh().sqrt();
    let k = kh / h;
    let cg = 0.5 * (1.0 + (2.0 * kh / KDMAX.min(2.0 * kh).sinh())) * si / k;

    (k, cg)
}

// Chen and Thomson wavenumber approximation.
pub fn wavenuma(angle_freq: f64, water_depth: f64) -> f64 {
    let koh = 0.10194 * angle_freq * angle_freq * water_depth;
    let d = [0.0, 0.6522, 0.4622, 0.0, 0.0864, 0.0675];
    let mut a = 1.0;
    for i in 1..d.len() {
        a += d[i] * koh.powi(i as i32);
    }

    (koh * (1.0 + 1.0 / (koh * a)).sqrt()) / water_depth
}

// Wave celerity C
// When depth is not supplied use deep water approximation
pub fn celerity(freq: f64, depth: Option<f64>) -> f64 {
    if let Some(depth) = depth {
        let angle_freq = 2.0 * PI * freq;
        angle_freq / wavenuma(angle_freq, depth)
    } else {
        1.56 / freq
    }
}

// Wavelength L
// When depth is not suppliked use deep water approximation
pub fn wavelength(freq: f64, depth: Option<f64>) -> f64 {
    if let Some(depth) = depth {
        let angle_freq = 2.0 * PI * freq;
        return 2.0 * PI / wavenuma(angle_freq, depth);
    } else {
        1.56 / freq.powi(2)
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
    partition_map: &[i32],
    frequency: &[f64],
    direction: &[f64],
    energy: &[f64],
    _dk: &[f64],
    dth: &[f64],
    depth: Option<f64>,
    wind_speed: Option<f64>,
    wind_direction: Option<f64>,
    source_direction_convention: &DirectionConvention,
) -> (Swell, Vec<Swell>) {
    const TPI: f64 = 2.0 * PI;
    let dera = 1.0f64.atan() / 45.0;
    const WSMULT: f64 = 1.7;
    
    let sig = (0..frequency.len() + 2)
        .map(|ik| {
            if ik == 0 {
                let diff = frequency[ik + 1] - frequency[ik];
                (frequency[ik] - diff) * 2.0 * PI
            } else if ik == frequency.len() + 1 {
                let diff = frequency[ik - 2] - frequency[ik - 3];
                (frequency[ik - 2] + diff) * 2.0 * PI
            } else {
                frequency[ik - 1] * 2.0 * PI
            }
        })
        .collect::<Vec<f64>>();

    let dsip = sig
        .iter()
        .enumerate()
        .map(|(ik, s)| {
            let xfr = if ik == 0 {
                frequency[ik + 1] / frequency[ik]
            } else if ik == 1 {
                frequency[ik] / frequency[ik - 1]
            } else if ik == frequency.len() + 1 {
                frequency[ik - 2] / frequency[ik - 3]
            } else {
                frequency[ik - 1] / frequency[ik - 2]
            };
            let sxfr = 0.5 * (xfr - 1. / xfr); // 0.06771
            s * sxfr
        })
        .collect::<Vec<f64>>();

    let mut dsii = vec![0.0; frequency.len()];
    dsii[0] = 0.5 * sig[1] * ((frequency[1] / frequency[0]) - 1.0);
    for ik in 1..dsii.len() - 1 {
        dsii[ik] = dsip[ik];
    }
    dsii[frequency.len() - 1] = 0.5 * sig[frequency.len()] * ((frequency[frequency.len() - 1] / frequency[frequency.len() - 2])  - 1.) / (frequency[frequency.len() - 1] / frequency[frequency.len() - 2]);

    let fte = 0.25 * sig[frequency.len()] * dth[dth.len() - 1] * sig[frequency.len()];

    let wn = sig[1..]
        .iter()
        .map(|s| match depth {
            Some(h) => wavenu3(*s, h).0,
            None => TPI / wavelength(*s, None),
        })
        .collect::<Vec<f64>>();

    let c = (0..frequency.len())
        .map(|i| sig[i + 1] / wn[i])
        .collect::<Vec<f64>>();

    let c_nk = c[c.len() - 1];

    let fcdir = direction
        .iter()
        .map(|th| {
            if let (Some(u_abs), Some(u_dir)) = (wind_speed, wind_direction) {
                let upar = WSMULT * u_abs * 0.0f64.max((th - dera * u_dir).cos());
                
                if upar < c_nk {
                    sig[sig.len() - 1]
                } else {
                    let mut ik = frequency.len() - 2;
                    while ik >= 1 {
                        if upar < c[ik] {
                            break;
                        }
    
                        ik = Sub::sub(ik, 1);
                    }
    
                    let mut rd = (c[ik] - upar) / (c[ik] - c[ik + 1]);
                    if rd < 0.0 {
                        ik = 0;
                        rd = 0.0f64.max(rd + 1.0);
                    }
    
                    // sig starts at 1 and goes to freqcount + 1
                    rd * sig[ik + 2] + (1.0 - rd) * sig[ik + 1]
                }
            } else {
                sig[sig.len() - 1]
            }
        })
        .collect::<Vec<f64>>();

    // Spectral integrals and preps
    // 3.a Integrals

    let mut sumf = vec![vec![0.0; num_partitions + 1]; frequency.len() + 2];
    let mut sumfw = vec![vec![0.0; num_partitions + 1]; frequency.len()];
    let mut sumfx = vec![vec![0.0; num_partitions + 1]; frequency.len()];
    let mut sumfy = vec![vec![0.0; num_partitions + 1]; frequency.len()];

    let (ecos, esin): (Vec<f64>, Vec<f64>) = direction
        .iter()
        .map(|d| {
            let mut ec = d.cos();
            let mut es = d.sin();
            if es.abs() < 1.0e-5 {
                es = 0.0;
                if ec > 0.5 {
                    ec = 1.0;
                } else {
                    ec = -1.0;
                }
            }

            if ec.abs() < 1.0e-5 {
                ec = 0.0;
                if es > 0.5 {
                    es = 1.0;
                } else {
                    es = -1.0;
                }
            }

            (ec, es)
        })
        .unzip();

    for ik in 0..frequency.len() {
        for ith in 0..direction.len() {
            let isp = ik + (ith * frequency.len());
            let ip = partition_map[isp]; // imo[ip]
            let fact = 0.0f64.max(
                1.0f64.min(1.0 - (fcdir[ith] - 0.05 * (sig[ik] + sig[ik + 1]) / dsip[ik + 1])),
            );

            sumf[ik][0] += energy[isp];
            sumfw[ik][0] += energy[isp] * fact;
            sumfx[ik][0] += energy[isp] * ecos[ith];
            sumfy[ik][0] += energy[isp] * esin[ith];

            if ip < 1 {
                continue;
            }

            sumf[ik][ip as usize] += energy[isp];
            sumfw[ik][ip as usize] += energy[isp] * fact;
            sumfx[ik][ip as usize] += energy[isp] * ecos[ith];
            sumfy[ik][ip as usize] += energy[isp] * esin[ith];
        }
    }

    // SUMF(NK+1,:) = SUMF(NK,:) * FACHFE

    let mut sume = vec![0.0; num_partitions + 1];
    let mut sume1 = vec![0.0; num_partitions + 1];
    let mut sume2 = vec![0.0; num_partitions + 1];
    let mut sumem1 = vec![0.0; num_partitions + 1];
    let mut sumew = vec![0.0; num_partitions + 1];
    let mut sumex = vec![0.0; num_partitions + 1];
    let mut sumey = vec![0.0; num_partitions + 1];
    let mut sumqp = vec![0.0; num_partitions + 1];
    let mut efpmax = vec![0.0; num_partitions + 1];
    let mut ifpmax = vec![0; num_partitions + 1];

    for ip in 0..num_partitions + 1 {
        for ik in 0..frequency.len() {
            sume[ip] += sumf[ik][ip] * dsii[ik];
            sumqp[ip] += sumf[ik][ip].powf(2.0) * dsii[ik] * sig[ik + 1];
            sume1[ip] += sumf[ik][ip] * dsii[ik] * sig[ik + 1];
            sume2[ip] += sumf[ik][ip] * dsii[ik] * sig[ik + 1].powf(2.0);
            sumem1[ip] += sumf[ik][ip] * dsii[ik] / sig[ik + 1];

            sumew[ip] += sumfw[ik][ip] * dsii[ik];
            sumex[ip] += sumfx[ik][ip] * dsii[ik];
            sumey[ip] += sumfy[ik][ip] * dsii[ik];

            if sumf[ik][ip] > efpmax[ip] {
                ifpmax[ip] = ik;
                efpmax[ip] = sumf[ik][ip];
            }
        }

        let fteii = fte / (dth[dth.len() - 1] * sig[frequency.len() + 1]);
        sume[ip] += sumf[frequency.len() - 1][ip] * fteii;
        sume1[ip] += sumf[frequency.len() - 1][ip] * sig[frequency.len()] * fteii * (0.3333 / 0.25);
        sume2[ip] +=
            sumf[frequency.len() - 1][ip] * sig[frequency.len()].powi(2) * fteii * (0.5 / 0.25);
        sumem1[ip] += sumf[frequency.len() - 1][ip] / sig[frequency.len()] * fteii * (0.2 / 0.25);
        sumqp[ip] += sumf[frequency.len() - 1][ip] * fteii;
        sumew[ip] += sumfw[frequency.len() - 1][ip] * fteii;
        sumex[ip] += sumfx[frequency.len() - 1][ip] * fteii;
        sumey[ip] += sumfy[frequency.len() - 1][ip] * fteii;
    }

    // Compute pars
    let mut components: Vec<Swell> = Vec::new();
    let mut summary: Swell = Swell::new(&UnitSystem::Metric, 0.0, 0.0, Direction::from_degrees(0), None);

    for ip in 0..num_partitions + 1 {
        let mo = sume[ip] * dth[0] * 1.0 / TPI;
        let hs = 4. * mo.max(0.0).sqrt();

        // If the derived swell height is too small, thow it away
        if ip > 0 && hs < 0.1 {
            continue;
        }

        let peak_period = TPI / sig[ifpmax[ip] + 1];

        // This calculates the direction towards, not from
        let raw_mean_wave_direction = f64::atan2(sumey[ip], sumex[ip]).to_degrees();
        let mean_wave_direction = match source_direction_convention {
            DirectionConvention::Met => (270.0 - raw_mean_wave_direction) % 360.0,
            DirectionConvention::From => (360.0 + raw_mean_wave_direction) % 360.0,
            DirectionConvention::Towards => (180.0 + raw_mean_wave_direction) % 360.0,
        };
        // let mean_wave_direction = (f64::atan2(sumey[ip], sumex[ip]).to_degrees()) % 360.0;
        // let sumexp = sumfx[ifpmax[ip]][ip] * dsii[ifpmax[ip]];
        // let sumeyp = sumfy[ifpmax[ip]][ip] * dsii[ifpmax[ip]];
        // let peak_wave_direction = (270.0 - f64::atan2(sumeyp, sumexp).to_degrees()) % 360.0;

        // Parabolic fit around the spectral peak
        let mut energy = sumf[ifpmax[ip]][ip] * dth[0];
        if ifpmax[ip] > 0 && ifpmax[ip] < frequency.len() - 1 {
            let el = sumf[ifpmax[ip] - 1][ip] * dth[0];
            let eh = sumf[ifpmax[ip] + 1][ip] * dth[0];
            let numer = 0.125 * (el - eh).powf(2.0);
            let denom = el - 2.0 * energy + eh;
            if denom != 0.0 {
                energy = energy - numer / denom.abs().copysign(denom);
            }
        }

        // let wind_sea_fraction = sumew[ip] / sume[ip];

        let component = Swell::new(
            &UnitSystem::Metric,
            hs,
            peak_period,
            Direction::from_degrees(mean_wave_direction as i32),
            Some(energy),
        );

        if ip == 0 {
            summary = component;
        } else {
            components.push(component);
        }

        components.sort_by(|sl, sr| sr.energy.partial_cmp(&sl.energy).unwrap());
    }

    (summary, components)
}
