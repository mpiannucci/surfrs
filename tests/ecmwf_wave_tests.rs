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
