use std::{collections::BTreeMap, f64::consts::PI};

use chrono::{DateTime, Utc};
use gribberish::message::Message;
use serde::{Deserialize, Serialize};

use crate::{
    dimensional_data::DimensionalData,
    location::Location,
    tools::grid_sampler::{GridSampler, PointSample, SampleSource},
    units::{Direction, Unit, UnitConvertible, UnitSystem},
};

use super::{
    ecmwf_field::{ECMWFField, ECMWFMessageInfo, ForecastMember, ECMWF_WAVE_PERIOD_BANDS},
    parseable_data_record::DataRecordParsingError,
};

/// Significant height of the wave energy with periods in `[min_period, max_period)`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeriodBandHeight {
    /// Seconds
    pub min_period: u8,
    /// Seconds
    pub max_period: u8,
    pub height: DimensionalData<f64>,
}

/// ECMWF open-data wave forecast at a point, for one member and valid time.
///
/// Only what ECMWF publishes: totals, a single mean direction for the whole sea
/// state, and the energy in six period bands from 10 to 30 s. There are no swell
/// partitions and no per-band directions. Wind is filled in when 10 m wind
/// messages from the matching `oper`/`enfo` file are passed in too.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ECMWFWavePointDataRecord {
    pub reference_date: DateTime<Utc>,
    /// Valid time
    pub date: DateTime<Utc>,
    pub member: ForecastMember,
    pub significant_wave_height: DimensionalData<f64>,
    /// Mean direction of the whole sea state, coming from.
    pub mean_direction: DimensionalData<Direction>,
    /// Tm-1,0
    pub energy_period: DimensionalData<f64>,
    /// Tm02. Not published by AIFS.
    pub zero_crossing_period: DimensionalData<f64>,
    /// Not published by AIFS.
    pub peak_period: DimensionalData<f64>,
    /// The bands present in the messages, shortest periods first.
    pub period_bands: Vec<PeriodBandHeight>,
    /// 10 m wind speed.
    pub wind_speed: DimensionalData<f64>,
    /// 10 m wind direction, coming from.
    pub wind_direction: DimensionalData<Direction>,
    /// How the significant wave height was sampled at the location.
    pub sample_source: SampleSource,
}

impl ECMWFWavePointDataRecord {
    /// One record per member and valid time found in `messages`, sorted by member
    /// then time. Wave and 10 m wind messages can be mixed. Probability,
    /// bathymetry and drag fields are ignored.
    pub fn from_messages(
        messages: &[Message],
        location: &Location,
    ) -> Result<Vec<Self>, DataRecordParsingError> {
        let mut records = Self::from_messages_many(messages, std::slice::from_ref(location))?;
        Ok(records.pop().unwrap_or_default())
    }

    /// Like `from_messages` for many locations, decoding each message once.
    /// Returns one list of records per location, in the same order.
    pub fn from_messages_many(
        messages: &[Message],
        locations: &[Location],
    ) -> Result<Vec<Vec<Self>>, DataRecordParsingError> {
        // (member, valid time) -> field -> one sample per location
        let mut groups: BTreeMap<
            (ForecastMember, DateTime<Utc>),
            (DateTime<Utc>, BTreeMap<FieldKey, Vec<PointSample>>),
        > = BTreeMap::new();

        for message in messages {
            let Some(info) = ECMWFMessageInfo::from_message(message) else {
                continue;
            };
            let Some(key) = FieldKey::from_field(&info.field) else {
                continue;
            };

            let sampler = GridSampler::from_message(message).map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "failed to decode {:?}: {e:?}",
                    info.field
                ))
            })?;
            let samples = sampler.sample_many(locations, info.field.sample_method());

            groups
                .entry((info.member, info.valid_date))
                .or_insert_with(|| (info.reference_date, BTreeMap::new()))
                .1
                .insert(key, samples);
        }

        let mut records = vec![Vec::with_capacity(groups.len()); locations.len()];
        for ((member, date), (reference_date, fields)) in groups {
            let Some(heights) = fields.get(&FieldKey::SignificantHeight) else {
                return Err(DataRecordParsingError::KeyMissing(format!(
                    "swh for {member:?} at {date}"
                )));
            };

            for (index, location_records) in records.iter_mut().enumerate() {
                let value =
                    |key: FieldKey| fields.get(&key).and_then(|samples| samples[index].value);
                let wind = match (value(FieldKey::WindU), value(FieldKey::WindV)) {
                    (Some(u), Some(v)) => Some((
                        u.hypot(v),
                        // Meteorological convention: the direction the wind comes from.
                        (-u).atan2(-v).to_degrees().rem_euclid(360.0),
                    )),
                    _ => None,
                };

                let period_bands = ECMWF_WAVE_PERIOD_BANDS
                    .iter()
                    .filter_map(|(min_period, max_period)| {
                        let samples =
                            fields.get(&FieldKey::PeriodBand(*min_period, *max_period))?;
                        Some(PeriodBandHeight {
                            min_period: *min_period,
                            max_period: *max_period,
                            height: dimensional(
                                samples[index].value,
                                "period band wave height",
                                Unit::Meters,
                            ),
                        })
                    })
                    .collect();

                location_records.push(ECMWFWavePointDataRecord {
                    reference_date,
                    date,
                    member,
                    significant_wave_height: dimensional(
                        heights[index].value,
                        "significant wave height",
                        Unit::Meters,
                    ),
                    mean_direction: DimensionalData {
                        value: value(FieldKey::MeanDirection)
                            .map(|d| Direction::from_degrees(d.round() as i32 % 360)),
                        variable_name: "mean wave direction".into(),
                        unit: Unit::Degrees,
                    },
                    energy_period: dimensional(
                        value(FieldKey::EnergyPeriod),
                        "energy period",
                        Unit::Seconds,
                    ),
                    zero_crossing_period: dimensional(
                        value(FieldKey::ZeroCrossingPeriod),
                        "zero crossing period",
                        Unit::Seconds,
                    ),
                    peak_period: dimensional(
                        value(FieldKey::PeakPeriod),
                        "peak period",
                        Unit::Seconds,
                    ),
                    period_bands,
                    wind_speed: dimensional(
                        wind.map(|(speed, _)| speed),
                        "wind speed",
                        Unit::MetersPerSecond,
                    ),
                    wind_direction: DimensionalData {
                        value: wind.map(|(_, direction)| {
                            Direction::from_degrees(direction.round() as i32 % 360)
                        }),
                        variable_name: "wind direction".into(),
                        unit: Unit::Degrees,
                    },
                    sample_source: heights[index].source,
                });
            }
        }

        Ok(records)
    }

    /// Significant height of the energy with periods from 10 to 30 s,
    /// sqrt(Σ band²). None unless all six bands are present.
    pub fn long_period_height(&self) -> DimensionalData<f64> {
        DimensionalData {
            value: self.band_energy_sum().map(f64::sqrt),
            variable_name: "long period wave height".into(),
            unit: self.significant_wave_height.unit.clone(),
        }
    }

    /// Significant height of the energy outside the 10-30 s bands (in practice
    /// periods under 10 s), sqrt(max(0, Hs² - Σ band²)). None unless all six
    /// bands are present.
    pub fn short_period_height(&self) -> DimensionalData<f64> {
        let value = match (self.significant_wave_height.value, self.band_energy_sum()) {
            (Some(hs), Some(bands)) => Some((hs * hs - bands).max(0.0).sqrt()),
            _ => None,
        };
        DimensionalData {
            value,
            variable_name: "short period wave height".into(),
            unit: self.significant_wave_height.unit.clone(),
        }
    }

    /// Deep water wave energy flux (power) per metre of crest,
    /// ρg²·Hs²·Tm-1,0 / (64π), in kW/m.
    pub fn energy_flux(&self) -> DimensionalData<f64> {
        let height_unit = &self.significant_wave_height.unit;
        let value = match (self.significant_wave_height.value, self.energy_period.value) {
            (Some(hs), Some(te)) => {
                let hs = height_unit.convert(hs, &Unit::Meters);
                Some(1029.0 * 9.81f64.powi(2) * hs * hs * te / (64.0 * PI) / 1000.0)
            }
            _ => None,
        };
        DimensionalData {
            value,
            variable_name: "wave energy flux".into(),
            unit: Unit::KiloWattsPerMeter,
        }
    }

    fn band_energy_sum(&self) -> Option<f64> {
        if self.period_bands.len() != ECMWF_WAVE_PERIOD_BANDS.len() {
            return None;
        }
        self.period_bands
            .iter()
            .map(|b| b.height.value.map(|h| h * h))
            .sum()
    }
}

impl UnitConvertible for ECMWFWavePointDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.significant_wave_height.to_units(new_units);
        self.mean_direction.to_units(new_units);
        self.energy_period.to_units(new_units);
        self.zero_crossing_period.to_units(new_units);
        self.peak_period.to_units(new_units);
        for band in &mut self.period_bands {
            band.height.to_units(new_units);
        }
        self.wind_speed.to_units(new_units);
        self.wind_direction.to_units(new_units);
        self
    }
}

/// The fields that make up a point record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FieldKey {
    SignificantHeight,
    MeanDirection,
    EnergyPeriod,
    ZeroCrossingPeriod,
    PeakPeriod,
    PeriodBand(u8, u8),
    WindU,
    WindV,
}

impl FieldKey {
    fn from_field(field: &ECMWFField) -> Option<Self> {
        match field {
            ECMWFField::SignificantHeight => Some(FieldKey::SignificantHeight),
            ECMWFField::MeanDirection => Some(FieldKey::MeanDirection),
            ECMWFField::EnergyPeriod => Some(FieldKey::EnergyPeriod),
            ECMWFField::ZeroCrossingPeriod => Some(FieldKey::ZeroCrossingPeriod),
            ECMWFField::PeakPeriod => Some(FieldKey::PeakPeriod),
            ECMWFField::PeriodBandHeight {
                min_period,
                max_period,
            } => Some(FieldKey::PeriodBand(*min_period, *max_period)),
            ECMWFField::WindU => Some(FieldKey::WindU),
            ECMWFField::WindV => Some(FieldKey::WindV),
            _ => None,
        }
    }
}

fn dimensional(value: Option<f64>, name: &str, unit: Unit) -> DimensionalData<f64> {
    DimensionalData {
        value,
        variable_name: name.into(),
        unit,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn record(hs: f64, bands: &[f64], energy_period: Option<f64>) -> ECMWFWavePointDataRecord {
        ECMWFWavePointDataRecord {
            reference_date: Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap(),
            date: Utc.with_ymd_and_hms(2026, 9, 24, 0, 0, 0).unwrap(),
            member: ForecastMember::Deterministic,
            significant_wave_height: dimensional(Some(hs), "significant wave height", Unit::Meters),
            mean_direction: DimensionalData {
                value: None,
                variable_name: "mean wave direction".into(),
                unit: Unit::Degrees,
            },
            energy_period: dimensional(energy_period, "energy period", Unit::Seconds),
            zero_crossing_period: dimensional(None, "zero crossing period", Unit::Seconds),
            peak_period: dimensional(None, "peak period", Unit::Seconds),
            period_bands: ECMWF_WAVE_PERIOD_BANDS
                .iter()
                .zip(bands)
                .map(|((min_period, max_period), h)| PeriodBandHeight {
                    min_period: *min_period,
                    max_period: *max_period,
                    height: dimensional(Some(*h), "period band wave height", Unit::Meters),
                })
                .collect(),
            wind_speed: dimensional(None, "wind speed", Unit::MetersPerSecond),
            wind_direction: DimensionalData {
                value: None,
                variable_name: "wind direction".into(),
                unit: Unit::Degrees,
            },
            sample_source: SampleSource::Interpolated { sea_cells: 4 },
        }
    }

    #[test]
    fn test_band_heights_split_energy() {
        // 3² = 1² + 2² + 2² + 0 + 0 + 0 + short²  =>  short = 0
        let r = record(3.0, &[1.0, 2.0, 2.0, 0.0, 0.0, 0.0], None);
        assert!((r.long_period_height().get_value() - 3.0).abs() < 1e-12);
        assert!(r.short_period_height().get_value().abs() < 1e-12);

        let r = record(5.0, &[0.0, 3.0, 0.0, 0.0, 0.0, 0.0], None);
        assert!((r.long_period_height().get_value() - 3.0).abs() < 1e-12);
        assert!((r.short_period_height().get_value() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_band_heights_need_all_bands() {
        let r = record(2.0, &[1.0, 1.0], None);
        assert_eq!(r.long_period_height().value, None);
        assert_eq!(r.short_period_height().value, None);
    }

    #[test]
    fn test_short_period_height_clamps_packing_noise() {
        let r = record(1.0, &[0.6, 0.6, 0.6, 0.0, 0.0, 0.0], None);
        assert_eq!(r.short_period_height().value, Some(0.0));
    }

    #[test]
    fn test_energy_flux() {
        // 1 m, 10 s: 1029 * 9.81² * 10 / (64π) / 1000 ≈ 4.925 kW/m (≈ 0.49 Hs² Te)
        let r = record(1.0, &[], Some(10.0));
        let flux = r.energy_flux();
        assert!((flux.get_value() - 4.925).abs() < 1e-3);
        assert_eq!(flux.unit, Unit::KiloWattsPerMeter);

        // Unit conversion of the record does not change the flux.
        let mut english = r.clone();
        english.to_units(&UnitSystem::English);
        assert_eq!(english.significant_wave_height.unit, Unit::Feet);
        assert!((english.energy_flux().get_value() - flux.get_value()).abs() < 1e-9);
    }
}
