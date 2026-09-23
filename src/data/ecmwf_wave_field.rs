use chrono::{DateTime, Utc};
use gribberish::{message::Message, templates::product::tables::ProbabilityType};
use serde::{Deserialize, Serialize};

use crate::{tools::grid_sampler::SampleMethod, units::Unit};

/// Lower and upper period (seconds) of the six ECMWF period-band height fields.
pub const ECMWF_WAVE_PERIOD_BANDS: [(u8, u8); 6] =
    [(10, 12), (12, 14), (14, 17), (17, 21), (21, 25), (25, 30)];

/// A wave field published in ECMWF open data (streams `wave` and `waef`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ECMWFWaveField {
    /// `swh`: significant height of combined wind waves and swell (m).
    SignificantHeight,
    /// `mwd`: mean direction of the whole sea state (degrees true, coming from).
    MeanDirection,
    /// `mwp`: mean period Tm-1,0, the energy period (s).
    EnergyPeriod,
    /// `mp2`: mean zero-crossing period Tm02 (s). IFS only.
    ZeroCrossingPeriod,
    /// `pp1d`: peak period (s). IFS only.
    PeakPeriod,
    /// `h1012` ... `h2530`: significant height of the energy with periods in
    /// `[min_period, max_period)` seconds (m).
    PeriodBandHeight { min_period: u8, max_period: u8 },
    /// `wmb`: model bathymetry (m, capped at 999).
    ModelBathymetry,
    /// `cdww`: drag coefficient with waves (dimensionless).
    DragCoefficient,
    /// `swhg2` ... `swhg8`: probability that Hs is at least `threshold` m (%).
    HeightExceedance { threshold: u8 },
    /// `mwpg8` ... `mwpg15`: probability that Tm-1,0 is at least `threshold` s (%). AIFS only.
    PeriodExceedance { threshold: u8 },
}

impl ECMWFWaveField {
    /// All six period-band height fields, shortest periods first.
    pub fn period_bands() -> Vec<ECMWFWaveField> {
        ECMWF_WAVE_PERIOD_BANDS
            .iter()
            .map(
                |(min_period, max_period)| ECMWFWaveField::PeriodBandHeight {
                    min_period: *min_period,
                    max_period: *max_period,
                },
            )
            .collect()
    }

    /// The MARS `param` short name used in the `.index` files.
    pub fn mars_param(&self) -> String {
        match self {
            ECMWFWaveField::SignificantHeight => "swh".into(),
            ECMWFWaveField::MeanDirection => "mwd".into(),
            ECMWFWaveField::EnergyPeriod => "mwp".into(),
            ECMWFWaveField::ZeroCrossingPeriod => "mp2".into(),
            ECMWFWaveField::PeakPeriod => "pp1d".into(),
            ECMWFWaveField::PeriodBandHeight {
                min_period,
                max_period,
            } => format!("h{min_period:02}{max_period:02}"),
            ECMWFWaveField::ModelBathymetry => "wmb".into(),
            ECMWFWaveField::DragCoefficient => "cdww".into(),
            ECMWFWaveField::HeightExceedance { threshold } => format!("swhg{threshold}"),
            ECMWFWaveField::PeriodExceedance { threshold } => format!("mwpg{threshold}"),
        }
    }

    /// How to sample the field at a point: heights as energy, directions as
    /// angles, everything else linearly.
    pub fn sample_method(&self) -> SampleMethod {
        match self {
            ECMWFWaveField::SignificantHeight | ECMWFWaveField::PeriodBandHeight { .. } => {
                SampleMethod::BilinearEnergy
            }
            ECMWFWaveField::MeanDirection => SampleMethod::BilinearAngular,
            _ => SampleMethod::Bilinear,
        }
    }

    pub fn unit(&self) -> Unit {
        match self {
            ECMWFWaveField::SignificantHeight
            | ECMWFWaveField::PeriodBandHeight { .. }
            | ECMWFWaveField::ModelBathymetry => Unit::Meters,
            ECMWFWaveField::MeanDirection => Unit::Degrees,
            ECMWFWaveField::EnergyPeriod
            | ECMWFWaveField::ZeroCrossingPeriod
            | ECMWFWaveField::PeakPeriod => Unit::Seconds,
            ECMWFWaveField::HeightExceedance { .. } | ECMWFWaveField::PeriodExceedance { .. } => {
                Unit::Percent
            }
            ECMWFWaveField::DragCoefficient => Unit::Unknown,
        }
    }

    /// Identify a decoded message.
    ///
    /// Uses the GRIB2 (discipline, category, number) plus the product template
    /// metadata. The abbreviation alone is not enough: the six period bands and the
    /// Hs probabilities all decode as `HTSGW`.
    pub fn from_message(message: &Message) -> Option<ECMWFWaveField> {
        let code = (
            message.discipline_value().ok()?,
            message.category_value().ok()?,
            message.parameter_value().ok()?,
        );
        let probability = exceedance_threshold(message);

        let field = match code {
            (10, 0, 3) => {
                if let Some(threshold) = probability {
                    ECMWFWaveField::HeightExceedance { threshold }
                } else if let Some((Some(min), Some(max))) = message.wave_period_range().ok()? {
                    ECMWFWaveField::PeriodBandHeight {
                        min_period: min.round() as u8,
                        max_period: max.round() as u8,
                    }
                } else {
                    ECMWFWaveField::SignificantHeight
                }
            }
            (10, 0, 14) => ECMWFWaveField::MeanDirection,
            (10, 0, 15) => match probability {
                Some(threshold) => ECMWFWaveField::PeriodExceedance { threshold },
                None => ECMWFWaveField::EnergyPeriod,
            },
            // AIFS ENS publishes the mean period probabilities with an ECMWF local
            // parameter rather than 10,0,15.
            (255, 131, 79) => ECMWFWaveField::PeriodExceedance {
                threshold: probability?,
            },
            (10, 0, 16) => ECMWFWaveField::DragCoefficient,
            (10, 0, 28) => ECMWFWaveField::ZeroCrossingPeriod,
            (10, 0, 34) => ECMWFWaveField::PeakPeriod,
            (10, 4, 7) => ECMWFWaveField::ModelBathymetry,
            _ => return None,
        };

        Some(field)
    }
}

/// The lower limit of an "above lower limit" probability product.
fn exceedance_threshold(message: &Message) -> Option<u8> {
    match message.probability_type().ok()? {
        Some(ProbabilityType::AboveLowerLimit) => message
            .probability_lower_limit()
            .ok()?
            .map(|limit| limit.round() as u8),
        _ => None,
    }
}

/// Which forecast in an ensemble a message belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ForecastMember {
    /// A single deterministic forecast (IFS HRES, AIFS single) or a product
    /// derived from the whole ensemble (probabilities).
    Deterministic,
    /// The unperturbed ensemble control (AIFS ENS `cf`).
    Control,
    /// A perturbed ensemble member, numbered from 1.
    Perturbed(u8),
}

impl ForecastMember {
    /// IFS HRES encodes its period bands as member 0 of an ensemble of 0, so an
    /// ensemble size of 0 means deterministic.
    pub fn from_message(message: &Message) -> ForecastMember {
        let members = message.number_of_ensemble_members().ok().flatten();
        let perturbation = message.perturbation_number().ok().flatten();
        match (members, perturbation) {
            (None | Some(0), _) | (_, None) => ForecastMember::Deterministic,
            (_, Some(0)) => ForecastMember::Control,
            (_, Some(n)) => ForecastMember::Perturbed(n),
        }
    }
}

/// A classified ECMWF wave message.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFWaveMessageInfo {
    pub field: ECMWFWaveField,
    pub member: ForecastMember,
    pub reference_date: DateTime<Utc>,
    /// Valid time, or the start of the window for time-window probabilities.
    pub valid_date: DateTime<Utc>,
    /// End of the window for time-window probabilities (e.g. step `120-168`).
    pub valid_end_date: Option<DateTime<Utc>>,
}

impl ECMWFWaveMessageInfo {
    pub fn from_message(message: &Message) -> Option<Self> {
        Some(ECMWFWaveMessageInfo {
            field: ECMWFWaveField::from_message(message)?,
            member: ForecastMember::from_message(message),
            reference_date: message.reference_date().ok()?,
            valid_date: message.forecast_date().ok()?,
            valid_end_date: message.forecast_end_date().ok().flatten(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mars_params() {
        let params: Vec<String> = ECMWFWaveField::period_bands()
            .iter()
            .map(|f| f.mars_param())
            .collect();
        assert_eq!(
            params,
            ["h1012", "h1214", "h1417", "h1721", "h2125", "h2530"]
        );
        assert_eq!(
            ECMWFWaveField::HeightExceedance { threshold: 2 }.mars_param(),
            "swhg2"
        );
        assert_eq!(
            ECMWFWaveField::PeriodExceedance { threshold: 10 }.mars_param(),
            "mwpg10"
        );
        assert_eq!(ECMWFWaveField::PeakPeriod.mars_param(), "pp1d");
    }

    #[test]
    fn test_sample_methods() {
        assert_eq!(
            ECMWFWaveField::SignificantHeight.sample_method(),
            SampleMethod::BilinearEnergy
        );
        assert_eq!(
            ECMWFWaveField::period_bands()[2].sample_method(),
            SampleMethod::BilinearEnergy
        );
        assert_eq!(
            ECMWFWaveField::MeanDirection.sample_method(),
            SampleMethod::BilinearAngular
        );
        assert_eq!(
            ECMWFWaveField::EnergyPeriod.sample_method(),
            SampleMethod::Bilinear
        );
    }
}
