use std::{collections::HashSet, fs};

use chrono::{Duration, TimeZone, Utc};
use gribberish::{index::parse_ecmwf_index, message::read_messages};
use surfrs::{
    data::ecmwf_field::{ECMWFField, ECMWFMessageInfo, ForecastMember},
    model::{ECMWFIndexQuery, MemberSelection},
};

// All fixtures are from the 2026-09-23 00z run, cut from gs://ecmwf-open-data.
const IFS_HRES: &str = "mock/ecmwf/ifs.20260923000000-24h-wave-fc";
const IFS_ENS: &str = "mock/ecmwf/ifs.20260923000000-24h-waef-ef";
const IFS_PROBABILITY: &str = "mock/ecmwf/ifs.20260923000000-240h-waef-ep";
const AIFS_SINGLE: &str = "mock/ecmwf/aifs-single.20260923000000-24h-wave-fc";
const AIFS_CONTROL: &str = "mock/ecmwf/aifs-ens.20260923000000-24h-waef-cf";
const AIFS_PROBABILITY: &str = "mock/ecmwf/aifs-ens.20260923000000-240h-waef-ep";

fn classify(stem: &str) -> Vec<ECMWFMessageInfo> {
    let data = fs::read(format!("{stem}.grib2")).expect("fixture not found");
    read_messages(&data)
        .map(|m| ECMWFMessageInfo::from_message(&m).expect("unclassified message"))
        .collect()
}

fn index(stem: &str) -> Vec<gribberish::index::IndexEntry> {
    parse_ecmwf_index(&fs::read_to_string(format!("{stem}.index")).unwrap()).unwrap()
}

fn band(min_period: u8, max_period: u8) -> ECMWFField {
    ECMWFField::PeriodBandHeight {
        min_period,
        max_period,
    }
}

#[test]
fn classifies_every_ifs_hres_field() {
    let infos = classify(IFS_HRES);
    let fields: HashSet<ECMWFField> = infos.iter().map(|i| i.field).collect();

    let mut expected: HashSet<ECMWFField> = ECMWFField::period_bands().into_iter().collect();
    expected.extend([
        ECMWFField::SignificantHeight,
        ECMWFField::MeanDirection,
        ECMWFField::EnergyPeriod,
        ECMWFField::ZeroCrossingPeriod,
        ECMWFField::PeakPeriod,
        ECMWFField::ModelBathymetry,
        ECMWFField::DragCoefficient,
    ]);
    assert_eq!(infos.len(), 13);
    assert_eq!(fields, expected);

    // The bands are encoded as member 0 of an ensemble of 0 but are deterministic.
    assert!(infos
        .iter()
        .all(|i| i.member == ForecastMember::Deterministic));

    let reference = Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap();
    assert!(infos.iter().all(|i| i.reference_date == reference
        && i.valid_date == reference + Duration::hours(24)
        && i.valid_end_date.is_none()));
}

#[test]
fn classifies_ensemble_members() {
    let mut found: Vec<(ECMWFField, ForecastMember)> = classify(IFS_ENS)
        .iter()
        .map(|i| (i.field, i.member))
        .collect();
    found.sort_by_key(|(_, member)| *member);
    assert_eq!(
        found.len(),
        3,
        "fixture holds swh for members 1-2 and h1417 for member 1"
    );
    assert!(found.contains(&(ECMWFField::SignificantHeight, ForecastMember::Perturbed(1))));
    assert!(found.contains(&(ECMWFField::SignificantHeight, ForecastMember::Perturbed(2))));
    assert!(found.contains(&(band(14, 17), ForecastMember::Perturbed(1))));

    let control = classify(AIFS_CONTROL);
    assert_eq!(control[0].field, band(14, 17));
    assert_eq!(control[0].member, ForecastMember::Control);
}

#[test]
fn classifies_aifs_period_bands() {
    // AIFS single encodes the bands with template 4.103 (no ensemble section).
    let infos = classify(AIFS_SINGLE);
    assert_eq!(infos[0].field, band(14, 17));
    assert_eq!(infos[0].member, ForecastMember::Deterministic);
}

#[test]
fn classifies_probabilities() {
    let ifs = classify(IFS_PROBABILITY);
    assert_eq!(ifs[0].field, ECMWFField::HeightExceedance { threshold: 2 });
    assert_eq!(ifs[0].member, ForecastMember::Deterministic);

    let reference = Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap();
    let aifs = classify(AIFS_PROBABILITY);
    let height = aifs
        .iter()
        .find(|i| i.field == ECMWFField::HeightExceedance { threshold: 2 })
        .unwrap();
    assert_eq!(height.valid_date, reference + Duration::hours(24));
    assert_eq!(height.valid_end_date, None);

    // mwpg10 over the 120-168h window, encoded with ECMWF local parameter 255/131/79.
    let period = aifs
        .iter()
        .find(|i| i.field == ECMWFField::PeriodExceedance { threshold: 10 })
        .unwrap();
    assert_eq!(period.valid_date, reference + Duration::hours(120));
    assert_eq!(
        period.valid_end_date,
        Some(reference + Duration::hours(168))
    );
}

#[test]
fn index_ranges_cut_the_requested_messages() {
    // The HRES fixture is the whole remote file, so index offsets apply to it directly.
    let data = fs::read(format!("{IFS_HRES}.grib2")).unwrap();
    let entries = index(IFS_HRES);

    let mut fields = vec![ECMWFField::SignificantHeight];
    fields.extend(ECMWFField::period_bands());
    let query = ECMWFIndexQuery {
        fields: fields.clone(),
        ..Default::default()
    };

    let ranges = query.select(&entries);
    assert_eq!(ranges.len(), 7);

    let decoded: HashSet<ECMWFField> = ranges
        .iter()
        .map(|r| {
            let bytes = &data[r.offset as usize..r.end() as usize];
            let message = read_messages(bytes).next().unwrap();
            ECMWFField::from_message(&message).unwrap()
        })
        .collect();
    assert_eq!(decoded, fields.into_iter().collect());

    // The six bands are adjacent in the file, so they merge into one request.
    let coalesced = query.select_coalesced(&entries);
    assert!(coalesced.len() < ranges.len());
    assert_eq!(
        coalesced.iter().map(|r| r.length).sum::<u64>(),
        ranges.iter().map(|r| r.length).sum::<u64>()
    );
}

#[test]
fn index_member_selection() {
    let entries = index(IFS_ENS);
    let swh_for = |members: MemberSelection| {
        ECMWFIndexQuery {
            fields: vec![ECMWFField::SignificantHeight],
            members,
            ..Default::default()
        }
        .select(&entries)
    };

    assert_eq!(swh_for(MemberSelection::All).len(), 50);
    assert_eq!(swh_for(MemberSelection::Only(vec![1, 2])).len(), 2);

    // Every field for one member.
    let query = ECMWFIndexQuery {
        members: MemberSelection::Only(vec![7]),
        ..Default::default()
    };
    assert_eq!(query.select(&entries).len(), 13);
}

#[test]
fn index_step_selection() {
    // Probability files hold every step, plus time windows for AIFS.
    let entries = index(AIFS_PROBABILITY);
    let query = ECMWFIndexQuery {
        fields: vec![ECMWFField::PeriodExceedance { threshold: 10 }],
        steps: Some(vec!["120-168".into()]),
        ..Default::default()
    };
    assert_eq!(query.select(&entries).len(), 1);

    let all_steps = ECMWFIndexQuery {
        fields: vec![ECMWFField::HeightExceedance { threshold: 2 }],
        ..Default::default()
    };
    assert!(all_steps.select(&entries).len() > 10);
}

mod point_record {
    use super::*;
    use surfrs::{
        data::ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
        location::Location,
        tools::grid_sampler::{GridSampler, SampleMethod, SampleSource},
        units::{Unit, UnitConvertible, UnitSystem},
    };

    fn messages(stem: &str) -> Vec<u8> {
        fs::read(format!("{stem}.grib2")).unwrap()
    }

    fn buoy_44097() -> Location {
        Location::new(40.97, -71.13, "NDBC 44097".into())
    }

    #[test]
    fn ifs_hres_record_at_44097() {
        let data = messages(IFS_HRES);
        let messages: Vec<_> = read_messages(&data).collect();
        let records = ECMWFWavePointDataRecord::from_messages(&messages, &buoy_44097()).unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];

        let reference = Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap();
        assert_eq!(record.reference_date, reference);
        assert_eq!(record.date, reference + Duration::hours(24));
        assert_eq!(record.member, ForecastMember::Deterministic);
        assert_eq!(
            record.sample_source,
            SampleSource::Interpolated { sea_cells: 4 }
        );

        // The record samples swh the same way GridSampler does on its own.
        let swh = messages
            .iter()
            .find(|m| ECMWFField::from_message(m) == Some(ECMWFField::SignificantHeight))
            .unwrap();
        let expected = GridSampler::from_message(swh)
            .unwrap()
            .sample(&buoy_44097(), SampleMethod::BilinearEnergy)
            .value
            .unwrap();
        assert_eq!(record.significant_wave_height.value, Some(expected));
        assert_eq!(record.significant_wave_height.unit, Unit::Meters);

        // Every field is present and the periods are ordered Tp >= Tm-1,0 >= Tm02.
        let tp = record.peak_period.value.unwrap();
        let te = record.energy_period.value.unwrap();
        let tz = record.zero_crossing_period.value.unwrap();
        assert!(tp >= te && te >= tz, "Tp {tp} Te {te} Tz {tz}");
        assert!(record.mean_direction.value.is_some());

        let bands: Vec<(u8, u8)> = record
            .period_bands
            .iter()
            .map(|b| (b.min_period, b.max_period))
            .collect();
        assert_eq!(bands, surfrs::data::ecmwf_field::ECMWF_WAVE_PERIOD_BANDS);

        let hs = expected;
        let long = record.long_period_height().value.unwrap();
        let short = record.short_period_height().value.unwrap();
        assert!((long * long + short * short - hs * hs).abs() < 1e-9);
        assert!(record.energy_flux().value.unwrap() > 0.0);
    }

    #[test]
    fn band_energy_never_exceeds_total_energy() {
        // Energy-weighted sampling keeps Hs² >= Σ band² wherever it holds at the
        // grid cells, so it should hold everywhere up to packing precision.
        let data = messages(IFS_HRES);
        let messages: Vec<_> = read_messages(&data).collect();
        let locations: Vec<Location> = (-70..=70)
            .flat_map(|lat| {
                (0..360)
                    .map(move |lng| Location::new(lat as f64 + 0.37, lng as f64 + 0.61, "".into()))
            })
            .collect();

        let records = ECMWFWavePointDataRecord::from_messages_many(&messages, &locations).unwrap();
        assert_eq!(records.len(), locations.len());

        let mut ocean_points = 0;
        let mut worst: f64 = 0.0;
        for record in records.iter().map(|r| &r[0]) {
            let (Some(hs), Some(long)) = (
                record.significant_wave_height.value,
                record.long_period_height().value,
            ) else {
                continue;
            };
            ocean_points += 1;
            worst = worst.max(long * long - hs * hs);
        }
        assert!(ocean_points > 25_000, "only {ocean_points} ocean points");
        assert!(worst < 1e-3, "band energy exceeds total by {worst} m²");
    }

    #[test]
    fn ensemble_members_become_separate_records() {
        let data = messages(IFS_ENS);
        let messages: Vec<_> = read_messages(&data).collect();
        let records = ECMWFWavePointDataRecord::from_messages(&messages, &buoy_44097()).unwrap();

        let members: Vec<ForecastMember> = records.iter().map(|r| r.member).collect();
        assert_eq!(
            members,
            vec![ForecastMember::Perturbed(1), ForecastMember::Perturbed(2)]
        );
        // The fixture has h1417 for member 1 only, and neither has every band.
        assert_eq!(records[0].period_bands.len(), 1);
        assert_eq!(records[0].period_bands[0].min_period, 14);
        assert!(records[1].period_bands.is_empty());
        assert_eq!(records[0].long_period_height().value, None);
        assert!(records[0].significant_wave_height.value.is_some());
        assert_eq!(records[0].peak_period.value, None);
    }

    #[test]
    fn many_locations_match_single_location() {
        let data = messages(IFS_HRES);
        let messages: Vec<_> = read_messages(&data).collect();
        let locations = vec![
            buoy_44097(),
            Location::new(40.1, -74.3, "Jackson, NJ".into()),
            Location::new(38.5, -98.0, "Kansas".into()),
        ];
        let many = ECMWFWavePointDataRecord::from_messages_many(&messages, &locations).unwrap();
        for (location, records) in locations.iter().zip(&many) {
            let single = ECMWFWavePointDataRecord::from_messages(&messages, location).unwrap();
            assert_eq!(
                serde_json::to_string(&single).unwrap(),
                serde_json::to_string(records).unwrap()
            );
        }

        // Inland: a record with no values, marked missing.
        let kansas = &many[2][0];
        assert_eq!(kansas.sample_source, SampleSource::Missing);
        assert_eq!(kansas.significant_wave_height.value, None);
        assert_eq!(kansas.energy_flux().value, None);
    }

    #[test]
    fn records_convert_units() {
        let data = messages(IFS_HRES);
        let messages: Vec<_> = read_messages(&data).collect();
        let mut record = ECMWFWavePointDataRecord::from_messages(&messages, &buoy_44097())
            .unwrap()
            .remove(0);
        let metres = record.significant_wave_height.value.unwrap();
        let flux = record.energy_flux().value.unwrap();

        record.to_units(&UnitSystem::English);
        assert_eq!(record.significant_wave_height.unit, Unit::Feet);
        assert!((record.significant_wave_height.value.unwrap() - metres * 3.28084).abs() < 1e-3);
        assert!(record
            .period_bands
            .iter()
            .all(|b| b.height.unit == Unit::Feet));
        assert_eq!(record.energy_period.unit, Unit::Seconds);
        assert!((record.energy_flux().value.unwrap() - flux).abs() < 1e-6);
    }
}

mod wind {
    use super::*;
    use surfrs::{
        data::ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
        location::Location,
        tools::grid_sampler::{GridSampler, SampleMethod},
    };

    const IFS_HRES_WIND: &str = "mock/ecmwf/ifs.20260923000000-24h-oper-fc";

    #[test]
    fn classifies_10m_wind() {
        let fields: HashSet<(ECMWFField, ForecastMember)> = classify(IFS_HRES_WIND)
            .iter()
            .map(|i| (i.field, i.member))
            .collect();
        assert_eq!(
            fields,
            HashSet::from([
                (ECMWFField::WindU, ForecastMember::Deterministic),
                (ECMWFField::WindV, ForecastMember::Deterministic),
            ])
        );
    }

    #[test]
    fn index_selects_10m_wind_only() {
        // The oper index also has u/v on pressure levels (params `u`/`v`) and at 100 m.
        let entries = index(IFS_HRES_WIND);
        let query = ECMWFIndexQuery {
            fields: vec![ECMWFField::WindU, ECMWFField::WindV],
            ..Default::default()
        };
        let ranges = query.select(&entries);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn record_combines_wave_and_wind_messages() {
        let wave = fs::read(format!("{IFS_HRES}.grib2")).unwrap();
        let wind = fs::read(format!("{IFS_HRES_WIND}.grib2")).unwrap();
        let messages: Vec<_> = read_messages(&wave).chain(read_messages(&wind)).collect();
        let location = Location::new(40.97, -71.13, "NDBC 44097".into());

        let records = ECMWFWavePointDataRecord::from_messages(&messages, &location).unwrap();
        assert_eq!(
            records.len(),
            1,
            "wind joins the wave record for the same time"
        );
        let record = &records[0];

        let component = |field: ECMWFField| {
            let message = messages
                .iter()
                .find(|m| ECMWFField::from_message(m) == Some(field))
                .unwrap();
            GridSampler::from_message(message)
                .unwrap()
                .sample(&location, SampleMethod::Bilinear)
                .value
                .unwrap()
        };
        let (u, v) = (component(ECMWFField::WindU), component(ECMWFField::WindV));
        assert!((record.wind_speed.value.unwrap() - u.hypot(v)).abs() < 1e-12);

        // eccodes nearest point: 10u -12.15, 10v -8.79, i.e. ~15 m/s from the northeast.
        let direction = record.wind_direction.value.as_ref().unwrap().degrees;
        assert!((40..=70).contains(&direction), "{direction}");
        assert!(record.significant_wave_height.value.is_some());
    }
}

mod ensemble {
    use super::*;
    use surfrs::{
        data::{
            ecmwf_wave_ensemble_point_data_record::{
                exceedance_probability, ECMWFWaveEnsemblePointDataRecord,
            },
            ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
        },
        location::Location,
    };

    fn member_records() -> Vec<ECMWFWavePointDataRecord> {
        let data = fs::read(format!("{IFS_ENS}.grib2")).unwrap();
        let messages: Vec<_> = read_messages(&data).collect();
        ECMWFWavePointDataRecord::from_messages(
            &messages,
            &Location::new(40.97, -71.13, "NDBC 44097".into()),
        )
        .unwrap()
    }

    #[test]
    fn statistics_from_member_records() {
        let members = member_records();
        let ensembles = ECMWFWaveEnsemblePointDataRecord::from_member_records(&members).unwrap();
        assert_eq!(ensembles.len(), 1);
        let ensemble = &ensembles[0];
        assert_eq!(ensemble.member_count, 2);
        assert_eq!(ensemble.date, members[0].date);

        let heights: Vec<f64> = members
            .iter()
            .map(|m| m.significant_wave_height.value.unwrap())
            .collect();
        let hs = ensemble.significant_wave_height.as_ref().unwrap();
        assert_eq!(hs.member_count, 2);
        assert!((hs.mean - (heights[0] + heights[1]) / 2.0).abs() < 1e-12);
        assert!((hs.spread - (heights[0] - heights[1]).abs() / 2.0).abs() < 1e-12);

        // Only member 1 has the 14-17 s band, and no member has all six bands.
        assert_eq!(ensemble.period_bands.len(), 1);
        assert_eq!(ensemble.period_bands[0].height.member_count, 1);
        assert!(ensemble.long_period_height.is_none());
        // No mwd or wind in the fixture.
        assert!(ensemble.mean_direction.is_none());
        assert!(ensemble.wind_speed.is_none());
    }

    #[test]
    fn deterministic_records_are_not_members() {
        let data = fs::read(format!("{IFS_HRES}.grib2")).unwrap();
        let messages: Vec<_> = read_messages(&data).collect();
        let hres = ECMWFWavePointDataRecord::from_messages(
            &messages,
            &Location::new(40.97, -71.13, "NDBC 44097".into()),
        )
        .unwrap();
        assert!(ECMWFWaveEnsemblePointDataRecord::from_members(&hres).is_err());
        assert!(ECMWFWaveEnsemblePointDataRecord::from_member_records(&hres)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn exceedance_from_members() {
        let members = member_records();
        let low = members
            .iter()
            .map(|m| m.significant_wave_height.value.unwrap())
            .fold(f64::INFINITY, f64::min);
        let p =
            exceedance_probability(&members, |m| m.significant_wave_height.value.unwrap() > low);
        assert_eq!(p, Some(0.5));
        assert_eq!(exceedance_probability(&[], |_| true), None);
    }
}

mod fitted_spectrum {
    use super::*;
    use surfrs::{
        data::{
            ecmwf_field::ECMWF_WAVE_PERIOD_BANDS,
            ecmwf_fitted_spectrum::{
                ECMWFFittedSpectrum, ECMWFWaveParameters, FitQuality, ECWAM_MAX_FREQUENCY,
            },
            ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
        },
        location::Location,
        units::{UnitConvertible, UnitSystem},
    };

    fn record_at_44097() -> ECMWFWavePointDataRecord {
        let data = fs::read(format!("{IFS_HRES}.grib2")).unwrap();
        let messages: Vec<_> = read_messages(&data).collect();
        let location = Location::new(40.97, -71.13, "NDBC 44097".into());
        ECMWFWavePointDataRecord::from_messages(&messages, &location)
            .unwrap()
            .remove(0)
    }

    fn fit(p: &ECMWFWaveParameters) -> ECMWFFittedSpectrum {
        p.fitted_spectrum(ECWAM_MAX_FREQUENCY).unwrap()
    }

    /// Hs from the bands at their true widths plus the fitted part over its
    /// whole range (midpoint rule in ln f).
    fn reconstructed_hs(spectrum: &ECMWFFittedSpectrum) -> f64 {
        let bands: f64 = ECMWF_WAVE_PERIOD_BANDS
            .iter()
            .zip(spectrum.band_density)
            .map(|((min, max), d)| d * (1.0 / *min as f64 - 1.0 / *max as f64))
            .sum();
        let n = 20_000;
        let (a, b) = (0.1f64.ln(), spectrum.max_frequency.ln());
        let h = (b - a) / n as f64;
        let short: f64 = (0..n)
            .map(|i| {
                let f = (a + h * (i as f64 + 0.5)).exp();
                spectrum.density(f) * f * h
            })
            .sum();
        4.0 * (bands + short).sqrt()
    }

    #[test]
    fn band_densities_are_exact_at_44097() {
        let record = record_at_44097();
        let spectrum = record.fitted_spectrum().unwrap();
        for (i, band) in record.period_bands.iter().enumerate() {
            let h = band.height.get_value();
            let (min, max) = (band.min_period as f64, band.max_period as f64);
            let expected = h * h / 16.0 / (1.0 / min - 1.0 / max);
            assert_eq!(spectrum.band_density[i], expected);
            // Any frequency inside the band reads the band average.
            assert_eq!(spectrum.density(2.0 / (min + max)), expected);
        }
    }

    #[test]
    fn curve_integrates_to_published_hs() {
        let p = ECMWFWaveParameters::from_record(&record_at_44097()).unwrap();
        let spectrum = fit(&p);
        let hs = p.significant_wave_height;
        assert!((reconstructed_hs(&spectrum) / hs - 1.0).abs() < 1e-4);

        // Sampled on a caller's axis (band midpoints, then the GFS grid to
        // 0.49 Hz), the tail above the axis is missing but little else.
        let mut axis: Vec<(f64, f64)> = ECMWF_WAVE_PERIOD_BANDS
            .iter()
            .map(|(min, max)| {
                (
                    2.0 / (*min + *max) as f64,
                    1.0 / *min as f64 - 1.0 / *max as f64,
                )
            })
            .collect();
        let r = 1.07f64.sqrt();
        axis.extend((16..=39).map(|n| {
            let f = 0.035 * 1.07f64.powi(n);
            (f, f * (r - 1.0 / r))
        }));
        let m0: f64 = axis.iter().map(|(f, df)| spectrum.density(*f) * df).sum();
        let axis_hs = 4.0 * m0.sqrt();
        assert!(axis_hs < hs && axis_hs / hs > 0.97, "{axis_hs} vs {hs}");
    }

    #[test]
    fn peak_anchored_puts_the_short_peak_at_tp() {
        let p = ECMWFWaveParameters::from_record(&record_at_44097()).unwrap();
        let tp = p.peak_period.unwrap();
        assert!(tp < 10.0, "fixture Tp {tp}");

        let spectrum = fit(&p);
        assert_eq!(spectrum.fit, FitQuality::PeakAnchored);
        assert!((spectrum.short_period.unwrap().peak_period - tp).abs() < 1e-9);
        let fp = 1.0 / tp;
        for ratio in [0.97, 0.99, 1.01, 1.03] {
            assert!(spectrum.density(fp * ratio) < spectrum.density(fp));
        }
    }

    #[test]
    fn approximate_without_a_short_tp() {
        let ifs = ECMWFWaveParameters::from_record(&record_at_44097()).unwrap();
        let ifs_fit = fit(&ifs).short_period.unwrap();

        // Tp in the swell: peak and γ both fitted.
        let long_tp = ECMWFWaveParameters {
            peak_period: Some(12.0),
            ..ifs.clone()
        };
        let spectrum = fit(&long_tp);
        assert_eq!(spectrum.fit, FitQuality::Approximate);
        assert!(spectrum.short_period.unwrap().peak_period < 10.0);
        assert!((reconstructed_hs(&spectrum) / ifs.significant_wave_height - 1.0).abs() < 1e-4);

        // AIFS: only Tm-1,0, so γ is fixed at 3.3. Same energy, different shape.
        let aifs = ECMWFWaveParameters {
            peak_period: None,
            zero_crossing_period: None,
            ..ifs.clone()
        };
        let spectrum = fit(&aifs);
        let aifs_fit = spectrum.short_period.unwrap();
        assert_eq!(spectrum.fit, FitQuality::Approximate);
        assert_eq!(aifs_fit.gamma, 3.3);
        assert!((aifs_fit.energy - ifs_fit.energy).abs() < 1e-12);
        assert!((reconstructed_hs(&spectrum) / ifs.significant_wave_height - 1.0).abs() < 1e-4);
        assert!(
            (aifs_fit.peak_period - ifs_fit.peak_period).abs() < 1.5,
            "{aifs_fit:?}"
        );
    }

    #[test]
    fn residual_is_clamped_when_bands_hold_all_the_energy() {
        let p = ECMWFWaveParameters {
            significant_wave_height: 1.0,
            energy_period: 13.0,
            zero_crossing_period: Some(12.0),
            peak_period: Some(14.5),
            band_heights: [0.41; 6],
        };
        let spectrum = fit(&p);
        assert!(spectrum.short_period.is_none());
        assert_eq!(spectrum.density(0.2), 0.0);
        assert!(spectrum.band_density.iter().all(|d| *d > 0.0));
    }

    #[test]
    fn record_needs_every_field_and_any_units_give_metres() {
        let record = record_at_44097();
        let mut missing_band = record.clone();
        missing_band.period_bands.pop();
        assert!(missing_band.fitted_spectrum().is_none());
        let mut missing_period = record.clone();
        missing_period.energy_period.value = None;
        assert!(missing_period.fitted_spectrum().is_none());

        let mut english = record.clone();
        english.to_units(&UnitSystem::English);
        let (metric, converted) = (
            record.fitted_spectrum().unwrap(),
            english.fitted_spectrum().unwrap(),
        );
        for f in [0.04, 0.09, 0.12, 0.3] {
            let (a, b) = (metric.density(f), converted.density(f));
            assert!((a - b).abs() <= 1e-9 * a, "{a} vs {b} at {f} Hz");
        }
    }

    #[test]
    fn fits_every_ocean_point() {
        let data = fs::read(format!("{IFS_HRES}.grib2")).unwrap();
        let messages: Vec<_> = read_messages(&data).collect();
        let locations: Vec<Location> = (-35..=35)
            .flat_map(|lat| {
                (0..180).map(move |lng| {
                    Location::new(2.0 * lat as f64 + 0.37, 2.0 * lng as f64 + 0.61, "".into())
                })
            })
            .collect();
        let records = ECMWFWavePointDataRecord::from_messages_many(&messages, &locations).unwrap();

        let (mut ocean, mut empty, mut worst_empty) = (0, 0, 0f64);
        for record in records.iter().map(|r| &r[0]) {
            if record.significant_wave_height.value.is_none() {
                continue;
            }
            ocean += 1;
            let spectrum = record.fitted_spectrum().expect("no fit");
            let error = (reconstructed_hs(&spectrum) / record.significant_wave_height.get_value()
                - 1.0)
                .abs();
            if spectrum.short_period.is_none() {
                // Packing noise, or a residual too small to place.
                empty += 1;
                worst_empty = worst_empty.max(error);
            } else {
                assert!(error < 1e-3, "Hs off by {error} for {record:?}");
            }
        }
        assert!(ocean > 6_000, "only {ocean} ocean points");
        assert!(worst_empty < 0.005, "{worst_empty}");
        println!(
            "{ocean} ocean points, {empty} empty below 10 s (Hs off by ≤ {:.2}%)",
            100.0 * worst_empty
        );
    }
}
