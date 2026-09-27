use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    dimensional_data::DimensionalData,
    units::{Direction, Unit, UnitConvertible, UnitSystem},
};

use super::{
    ecmwf_field::{ForecastMember, ECMWF_WAVE_PERIOD_BANDS},
    ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
    parseable_data_record::DataRecordParsingError,
};

/// Summary of one quantity across ensemble members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnsembleStatistics {
    /// Members with a value for this quantity.
    pub member_count: usize,
    pub mean: f64,
    /// Population standard deviation across members.
    pub spread: f64,
    pub min: f64,
    pub max: f64,
    /// Percentiles use linear interpolation between members.
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub unit: Unit,
}

impl EnsembleStatistics {
    /// None when `values` is empty.
    pub fn from_values(values: &[f64], unit: Unit) -> Option<Self> {
        if values.is_empty() {
            return None;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = sorted.len() as f64;
        let mean = sorted.iter().sum::<f64>() / count;
        let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count;

        Some(EnsembleStatistics {
            member_count: sorted.len(),
            mean,
            spread: variance.sqrt(),
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            p10: percentile(&sorted, 0.1),
            p50: percentile(&sorted, 0.5),
            p90: percentile(&sorted, 0.9),
            unit,
        })
    }
}

impl UnitConvertible for EnsembleStatistics {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        let target = self.unit.convert_system(new_units);
        if target != self.unit {
            let unit = self.unit.clone();
            let convert = |v: f64| unit.convert(v, &target);
            self.mean = convert(self.mean);
            // Spread and the rest are linear in the unit for every unit used here.
            self.spread = convert(self.spread);
            self.min = convert(self.min);
            self.max = convert(self.max);
            self.p10 = convert(self.p10);
            self.p50 = convert(self.p50);
            self.p90 = convert(self.p90);
            self.unit = target;
        }
        self
    }
}

/// Summary of a direction across ensemble members, using circular statistics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AngularEnsembleStatistics {
    pub member_count: usize,
    /// Direction of the mean unit vector, degrees.
    pub mean: f64,
    /// Circular standard deviation sqrt(-2 ln R), degrees.
    pub spread: f64,
    /// Length of the mean unit vector, 0 (members spread all round) to 1 (all agree).
    pub resultant_length: f64,
}

impl AngularEnsembleStatistics {
    /// None when `degrees` is empty.
    pub fn from_degrees(degrees: &[f64]) -> Option<Self> {
        if degrees.is_empty() {
            return None;
        }

        let count = degrees.len() as f64;
        let (sin_sum, cos_sum) = degrees.iter().fold((0.0, 0.0), |(s, c), d| {
            let r = d.to_radians();
            (s + r.sin(), c + r.cos())
        });
        let resultant_length = ((sin_sum / count).powi(2) + (cos_sum / count).powi(2))
            .sqrt()
            .min(1.0);

        Some(AngularEnsembleStatistics {
            member_count: degrees.len(),
            mean: sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0),
            spread: (-2.0 * resultant_length.max(f64::MIN_POSITIVE).ln())
                .sqrt()
                .to_degrees(),
            resultant_length,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeriodBandStatistics {
    pub min_period: u8,
    pub max_period: u8,
    pub height: EnsembleStatistics,
}

/// Ensemble statistics for one location and valid time, computed from the
/// member records. ECMWF does not publish an ensemble mean or spread.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ECMWFWaveEnsemblePointDataRecord {
    pub reference_date: DateTime<Utc>,
    /// Valid time
    pub date: DateTime<Utc>,
    /// Member records used (control plus perturbed members).
    pub member_count: usize,
    pub significant_wave_height: Option<EnsembleStatistics>,
    pub mean_direction: Option<AngularEnsembleStatistics>,
    pub energy_period: Option<EnsembleStatistics>,
    pub zero_crossing_period: Option<EnsembleStatistics>,
    pub peak_period: Option<EnsembleStatistics>,
    pub period_bands: Vec<PeriodBandStatistics>,
    pub long_period_height: Option<EnsembleStatistics>,
    pub short_period_height: Option<EnsembleStatistics>,
    pub energy_flux: Option<EnsembleStatistics>,
    pub wind_speed: Option<EnsembleStatistics>,
    pub wind_direction: Option<AngularEnsembleStatistics>,
}

impl ECMWFWaveEnsemblePointDataRecord {
    /// Statistics across `members`, which must all be ensemble members (control
    /// or perturbed) for the same valid time. Each quantity uses the members that
    /// have a value for it; heights are converted to the first member's unit.
    pub fn from_members(
        members: &[ECMWFWavePointDataRecord],
    ) -> Result<Self, DataRecordParsingError> {
        let Some(first) = members.first() else {
            return Err(DataRecordParsingError::InvalidData);
        };
        if let Some(record) = members.iter().find(|m| m.date != first.date) {
            return Err(DataRecordParsingError::ParseFailure(format!(
                "members have different valid times: {} and {}",
                first.date, record.date
            )));
        }
        if members
            .iter()
            .any(|m| m.member == ForecastMember::Deterministic)
        {
            return Err(DataRecordParsingError::ParseFailure(
                "deterministic records are not ensemble members".into(),
            ));
        }

        let height_unit = first.significant_wave_height.unit.clone();
        let heights = |get: &dyn Fn(&ECMWFWavePointDataRecord) -> DimensionalData<f64>| {
            let values: Vec<f64> = members
                .iter()
                .filter_map(|m| {
                    let data = get(m);
                    data.value.map(|v| data.unit.convert(v, &height_unit))
                })
                .collect();
            EnsembleStatistics::from_values(&values, height_unit.clone())
        };
        let scalars = |get: &dyn Fn(&ECMWFWavePointDataRecord) -> DimensionalData<f64>| {
            let values: Vec<f64> = members.iter().filter_map(|m| get(m).value).collect();
            EnsembleStatistics::from_values(&values, get(first).unit)
        };
        let directions = |get: &dyn Fn(&ECMWFWavePointDataRecord) -> Option<Direction>| {
            let values: Vec<f64> = members
                .iter()
                .filter_map(|m| get(m).map(|d| d.degrees as f64))
                .collect();
            AngularEnsembleStatistics::from_degrees(&values)
        };

        let period_bands = ECMWF_WAVE_PERIOD_BANDS
            .iter()
            .filter_map(|(min_period, max_period)| {
                let band_height = |m: &ECMWFWavePointDataRecord| {
                    m.period_bands
                        .iter()
                        .find(|b| b.min_period == *min_period && b.max_period == *max_period)
                        .map(|b| b.height.clone())
                        .unwrap_or(DimensionalData {
                            value: None,
                            variable_name: "period band wave height".into(),
                            unit: height_unit.clone(),
                        })
                };
                Some(PeriodBandStatistics {
                    min_period: *min_period,
                    max_period: *max_period,
                    height: heights(&band_height)?,
                })
            })
            .collect();

        Ok(ECMWFWaveEnsemblePointDataRecord {
            reference_date: first.reference_date,
            date: first.date,
            member_count: members.len(),
            significant_wave_height: heights(&|m| m.significant_wave_height.clone()),
            mean_direction: directions(&|m| m.mean_direction.value.clone()),
            energy_period: scalars(&|m| m.energy_period.clone()),
            zero_crossing_period: scalars(&|m| m.zero_crossing_period.clone()),
            peak_period: scalars(&|m| m.peak_period.clone()),
            period_bands,
            long_period_height: heights(&|m| m.long_period_height()),
            short_period_height: heights(&|m| m.short_period_height()),
            energy_flux: scalars(&|m| m.energy_flux()),
            wind_speed: scalars(&|m| m.wind_speed.clone()),
            wind_direction: directions(&|m| m.wind_direction.value.clone()),
        })
    }

    /// Group member records by valid time and summarise each group, in time
    /// order. Deterministic records are skipped.
    pub fn from_member_records(
        records: &[ECMWFWavePointDataRecord],
    ) -> Result<Vec<Self>, DataRecordParsingError> {
        let mut by_date: BTreeMap<DateTime<Utc>, Vec<ECMWFWavePointDataRecord>> = BTreeMap::new();
        for record in records {
            if record.member != ForecastMember::Deterministic {
                by_date.entry(record.date).or_default().push(record.clone());
            }
        }
        by_date
            .values()
            .map(|members| Self::from_members(members))
            .collect()
    }
}

impl UnitConvertible for ECMWFWaveEnsemblePointDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        let stats = [
            &mut self.significant_wave_height,
            &mut self.energy_period,
            &mut self.zero_crossing_period,
            &mut self.peak_period,
            &mut self.long_period_height,
            &mut self.short_period_height,
            &mut self.energy_flux,
            &mut self.wind_speed,
        ];
        for stat in stats.into_iter().flatten() {
            stat.to_units(new_units);
        }
        for band in &mut self.period_bands {
            band.height.to_units(new_units);
        }
        self
    }
}

/// Fraction of `members` (0-1) for which `predicate` holds, e.g. the chance that
/// the 14-17 s band is above 0.5 m. None when there are no members.
pub fn exceedance_probability(
    members: &[ECMWFWavePointDataRecord],
    predicate: impl Fn(&ECMWFWavePointDataRecord) -> bool,
) -> Option<f64> {
    if members.is_empty() {
        return None;
    }
    Some(members.iter().filter(|m| predicate(m)).count() as f64 / members.len() as f64)
}

/// Linear interpolation between order statistics (the numpy default).
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let rank = q * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    sorted[lower] + (sorted[upper] - sorted[lower]) * (rank - lower as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics() {
        let stats =
            EnsembleStatistics::from_values(&[4.0, 1.0, 3.0, 2.0, 5.0], Unit::Meters).unwrap();
        assert_eq!(stats.member_count, 5);
        assert!((stats.mean - 3.0).abs() < 1e-12);
        assert!((stats.spread - 2.0_f64.sqrt()).abs() < 1e-12);
        assert_eq!((stats.min, stats.max), (1.0, 5.0));
        assert!((stats.p10 - 1.4).abs() < 1e-12);
        assert!((stats.p50 - 3.0).abs() < 1e-12);
        assert!((stats.p90 - 4.6).abs() < 1e-12);

        assert!(EnsembleStatistics::from_values(&[], Unit::Meters).is_none());
    }

    #[test]
    fn test_statistics_unit_conversion() {
        let mut stats = EnsembleStatistics::from_values(&[1.0, 3.0], Unit::Meters).unwrap();
        stats.to_units(&UnitSystem::English);
        assert_eq!(stats.unit, Unit::Feet);
        assert!((stats.mean - 2.0 * 3.28084).abs() < 1e-3);
        assert!((stats.spread - 3.28084).abs() < 1e-3);
    }

    #[test]
    fn test_angular_statistics_across_north() {
        let stats = AngularEnsembleStatistics::from_degrees(&[350.0, 10.0]).unwrap();
        assert!(stats.mean < 1e-9 || 360.0 - stats.mean < 1e-9);
        assert!(stats.spread > 9.0 && stats.spread < 11.0);

        let agree = AngularEnsembleStatistics::from_degrees(&[90.0, 90.0, 90.0]).unwrap();
        assert!((agree.mean - 90.0).abs() < 1e-9);
        assert!(agree.spread < 1e-6);
        assert!((agree.resultant_length - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_percentile_single_member() {
        let stats = EnsembleStatistics::from_values(&[2.5], Unit::Meters).unwrap();
        assert_eq!(
            (stats.p10, stats.p50, stats.p90, stats.spread),
            (2.5, 2.5, 2.5, 0.0)
        );
    }
}
