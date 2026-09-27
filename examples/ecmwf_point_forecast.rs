// Point forecast from ECMWF open data: fetch only the needed messages with
// HTTP range reads, then build point records.
//
//   cargo run --release --example ecmwf_point_forecast -- <lat> <lon>
//
// Environment:
//   MAX_HOUR     last forecast hour for the IFS HRES table (default 72)
//   ENS_HOURS    comma separated hours for IFS ENS statistics (default none).
//                Each hour downloads ~0.85 MB per field per member (~170 MB here).
use std::env;

use chrono::Utc;
use futures_util::StreamExt;
use gribberish::{index::parse_ecmwf_index, message::read_messages};
use reqwest::Client;
use surfrs::{
    data::{
        ecmwf_field::ECMWFField,
        ecmwf_wave_ensemble_point_data_record::{
            exceedance_probability, ECMWFWaveEnsemblePointDataRecord,
        },
        ecmwf_wave_point_data_record::ECMWFWavePointDataRecord,
    },
    location::Location,
    model::{
        ByteRange, ECMWFDataSource, ECMWFIndexQuery, ECMWFWaveModel, ECMWFWindModel, GriddedModel,
    },
};

const SOURCE: ECMWFDataSource = ECMWFDataSource::GCP;

/// Download the messages an index query selects from one file.
async fn fetch(
    client: &Client,
    index_url: &str,
    url: &str,
    query: &ECMWFIndexQuery,
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let index = client
        .get(index_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let entries = parse_ecmwf_index(&index)?;
    let ranges: Vec<ByteRange> = query.select_coalesced(&entries);

    let chunks = futures::stream::iter(ranges)
        .map(|range| async move {
            client
                .get(url)
                .header("Range", range.http_range_header())
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await
                .map(|b| b.to_vec())
        })
        .buffer_unordered(16)
        .collect::<Vec<_>>()
        .await;

    Ok(chunks.into_iter().collect::<Result<Vec<_>, _>>()?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (lat, lon) = match (args.get(1), args.get(2)) {
        (Some(lat), Some(lon)) => (lat.parse()?, lon.parse()?),
        _ => (40.97, -71.13), // NDBC 44097
    };
    let location = Location::new(lat, lon, "point".into());
    let max_hour: usize = env::var("MAX_HOUR")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(72);
    let ens_hours: Vec<usize> = env::var("ENS_HOURS")
        .map(|h| h.split(',').filter_map(|h| h.trim().parse().ok()).collect())
        .unwrap_or_default();

    let client = Client::new();
    let now = Utc::now();

    // IFS HRES wave fields plus 10 m wind from the matching oper file.
    let wave_model = ECMWFWaveModel::ifs_hres();
    let wind_model = ECMWFWindModel::for_wave_model(&wave_model).unwrap();
    let run = wave_model.closest_model_run_date(&now);
    println!("IFS HRES run {run} at {lat}, {lon}\n");
    println!(
        "{:<17} {:>5} {:>5} {:>5} {:>5} {:>4} {:>6} {:>6} {:>6} {:>5} {:>4}",
        "valid (UTC)", "Hs", "Tp", "Te", "Tz", "Dir", "Hs10+", "Hs<10", "kW/m", "Wind", "WDir"
    );

    let mut wave_fields = vec![
        ECMWFField::SignificantHeight,
        ECMWFField::MeanDirection,
        ECMWFField::EnergyPeriod,
        ECMWFField::ZeroCrossingPeriod,
        ECMWFField::PeakPeriod,
    ];
    wave_fields.extend(ECMWFField::period_bands());
    let wave_query = ECMWFIndexQuery {
        fields: wave_fields,
        ..Default::default()
    };
    let wind_query = ECMWFIndexQuery {
        fields: vec![ECMWFField::WindU, ECMWFField::WindV],
        ..Default::default()
    };

    for hour in wave_model
        .forecast_hours(&run)
        .into_iter()
        .filter(|h| *h <= max_hour)
    {
        let mut chunks = fetch(
            &client,
            &wave_model.create_index_url(&SOURCE, hour, Some(now)),
            &wave_model.create_url(&SOURCE, hour, Some(now)),
            &wave_query,
        )
        .await?;
        chunks.extend(
            fetch(
                &client,
                &wind_model.create_index_url(&SOURCE, hour, Some(now)),
                &wind_model.create_url(&SOURCE, hour, Some(now)),
                &wind_query,
            )
            .await?,
        );
        let messages: Vec<_> = chunks.iter().flat_map(|c| read_messages(c)).collect();

        for r in ECMWFWavePointDataRecord::from_messages(&messages, &location)? {
            println!(
                "{:<17} {:>5.2} {:>5.1} {:>5.1} {:>5.1} {:>4} {:>6.2} {:>6.2} {:>6.1} {:>5.1} {:>4}",
                r.date.format("%Y-%m-%d %H:%M"),
                r.significant_wave_height.get_value(),
                r.peak_period.get_value(),
                r.energy_period.get_value(),
                r.zero_crossing_period.get_value(),
                r.mean_direction.get_value().degrees,
                r.long_period_height().get_value(),
                r.short_period_height().get_value(),
                r.energy_flux().get_value(),
                r.wind_speed.get_value(),
                r.wind_direction.get_value().degrees,
            );
        }
    }

    if ens_hours.is_empty() {
        return Ok(());
    }

    // IFS ENS: all 50 members are in one file per step, but the messages are not
    // grouped by field or member, so each member's field is its own request.
    let ens_model = ECMWFWaveModel::ifs_ens_members();
    let ens_run = ens_model.closest_model_run_date(&now);
    let band = ECMWFField::PeriodBandHeight {
        min_period: 14,
        max_period: 17,
    };
    let ens_query = ECMWFIndexQuery {
        fields: vec![
            ECMWFField::SignificantHeight,
            ECMWFField::EnergyPeriod,
            ECMWFField::MeanDirection,
            band,
        ],
        ..Default::default()
    };

    println!("\nIFS ENS run {ens_run}");
    println!(
        "{:<17} {:>3} {:>12} {:>13} {:>11} {:>12} {:>13}",
        "valid (UTC)", "n", "Hs mean±sd", "Hs p10-p90", "Te mean", "Dir mean±sd", "P(14-17s>0.3m)"
    );
    for hour in ens_hours {
        let chunks = fetch(
            &client,
            &ens_model.create_index_url(&SOURCE, hour, Some(now)),
            &ens_model.create_url(&SOURCE, hour, Some(now)),
            &ens_query,
        )
        .await?;
        let messages: Vec<_> = chunks.iter().flat_map(|c| read_messages(c)).collect();
        let members = ECMWFWavePointDataRecord::from_messages(&messages, &location)?;

        for ensemble in ECMWFWaveEnsemblePointDataRecord::from_member_records(&members)? {
            let hs = ensemble.significant_wave_height.as_ref().unwrap();
            let te = ensemble.energy_period.as_ref().unwrap();
            let dir = ensemble.mean_direction.as_ref().unwrap();
            let at_time: Vec<_> = members
                .iter()
                .filter(|m| m.date == ensemble.date)
                .cloned()
                .collect();
            let p = exceedance_probability(&at_time, |m| {
                m.period_bands
                    .iter()
                    .any(|b| b.min_period == 14 && b.height.value.map_or(false, |h| h > 0.3))
            })
            .unwrap_or(f64::NAN);
            println!(
                "{:<17} {:>3} {:>5.2}±{:<5.2} {:>6.2}-{:<6.2} {:>11.1} {:>6.0}±{:<5.0} {:>12.0}%",
                ensemble.date.format("%Y-%m-%d %H:%M"),
                ensemble.member_count,
                hs.mean,
                hs.spread,
                hs.p10,
                hs.p90,
                te.mean,
                dir.mean,
                dir.spread,
                p * 100.0
            );
        }
    }

    Ok(())
}
