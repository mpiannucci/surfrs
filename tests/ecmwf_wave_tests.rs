use std::{collections::HashSet, fs};

use chrono::{Duration, TimeZone, Utc};
use gribberish::{index::parse_ecmwf_index, message::read_messages};
use surfrs::{
    data::ecmwf_wave_field::{ECMWFWaveField, ECMWFWaveMessageInfo, ForecastMember},
    model::{ECMWFIndexQuery, MemberSelection},
};

// All fixtures are from the 2026-09-23 00z run, cut from gs://ecmwf-open-data.
const IFS_HRES: &str = "mock/ecmwf/ifs.20260923000000-24h-wave-fc";
const IFS_ENS: &str = "mock/ecmwf/ifs.20260923000000-24h-waef-ef";
const IFS_PROBABILITY: &str = "mock/ecmwf/ifs.20260923000000-240h-waef-ep";
const AIFS_SINGLE: &str = "mock/ecmwf/aifs-single.20260923000000-24h-wave-fc";
const AIFS_CONTROL: &str = "mock/ecmwf/aifs-ens.20260923000000-24h-waef-cf";
const AIFS_PROBABILITY: &str = "mock/ecmwf/aifs-ens.20260923000000-240h-waef-ep";

fn classify(stem: &str) -> Vec<ECMWFWaveMessageInfo> {
    let data = fs::read(format!("{stem}.grib2")).expect("fixture not found");
    read_messages(&data)
        .map(|m| ECMWFWaveMessageInfo::from_message(&m).expect("unclassified message"))
        .collect()
}

fn index(stem: &str) -> Vec<gribberish::index::IndexEntry> {
    parse_ecmwf_index(&fs::read_to_string(format!("{stem}.index")).unwrap()).unwrap()
}

fn band(min_period: u8, max_period: u8) -> ECMWFWaveField {
    ECMWFWaveField::PeriodBandHeight {
        min_period,
        max_period,
    }
}

#[test]
fn classifies_every_ifs_hres_field() {
    let infos = classify(IFS_HRES);
    let fields: HashSet<ECMWFWaveField> = infos.iter().map(|i| i.field).collect();

    let mut expected: HashSet<ECMWFWaveField> =
        ECMWFWaveField::period_bands().into_iter().collect();
    expected.extend([
        ECMWFWaveField::SignificantHeight,
        ECMWFWaveField::MeanDirection,
        ECMWFWaveField::EnergyPeriod,
        ECMWFWaveField::ZeroCrossingPeriod,
        ECMWFWaveField::PeakPeriod,
        ECMWFWaveField::ModelBathymetry,
        ECMWFWaveField::DragCoefficient,
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
    let mut found: Vec<(ECMWFWaveField, ForecastMember)> = classify(IFS_ENS)
        .iter()
        .map(|i| (i.field, i.member))
        .collect();
    found.sort_by_key(|(_, member)| *member);
    assert_eq!(
        found.len(),
        3,
        "fixture holds swh for members 1-2 and h1417 for member 1"
    );
    assert!(found.contains(&(
        ECMWFWaveField::SignificantHeight,
        ForecastMember::Perturbed(1)
    )));
    assert!(found.contains(&(
        ECMWFWaveField::SignificantHeight,
        ForecastMember::Perturbed(2)
    )));
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
    assert_eq!(
        ifs[0].field,
        ECMWFWaveField::HeightExceedance { threshold: 2 }
    );
    assert_eq!(ifs[0].member, ForecastMember::Deterministic);

    let reference = Utc.with_ymd_and_hms(2026, 9, 23, 0, 0, 0).unwrap();
    let aifs = classify(AIFS_PROBABILITY);
    let height = aifs
        .iter()
        .find(|i| i.field == ECMWFWaveField::HeightExceedance { threshold: 2 })
        .unwrap();
    assert_eq!(height.valid_date, reference + Duration::hours(24));
    assert_eq!(height.valid_end_date, None);

    // mwpg10 over the 120-168h window, encoded with ECMWF local parameter 255/131/79.
    let period = aifs
        .iter()
        .find(|i| i.field == ECMWFWaveField::PeriodExceedance { threshold: 10 })
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

    let mut fields = vec![ECMWFWaveField::SignificantHeight];
    fields.extend(ECMWFWaveField::period_bands());
    let query = ECMWFIndexQuery {
        fields: fields.clone(),
        ..Default::default()
    };

    let ranges = query.select(&entries);
    assert_eq!(ranges.len(), 7);

    let decoded: HashSet<ECMWFWaveField> = ranges
        .iter()
        .map(|r| {
            let bytes = &data[r.offset as usize..r.end() as usize];
            let message = read_messages(bytes).next().unwrap();
            ECMWFWaveField::from_message(&message).unwrap()
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
            fields: vec![ECMWFWaveField::SignificantHeight],
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
        fields: vec![ECMWFWaveField::PeriodExceedance { threshold: 10 }],
        steps: Some(vec!["120-168".into()]),
        ..Default::default()
    };
    assert_eq!(query.select(&entries).len(), 1);

    let all_steps = ECMWFIndexQuery {
        fields: vec![ECMWFWaveField::HeightExceedance { threshold: 2 }],
        ..Default::default()
    };
    assert!(all_steps.select(&entries).len() > 10);
}
