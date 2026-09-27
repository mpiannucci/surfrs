// Score IFS HRES and GFS-Wave point forecasts against NDBC buoy observations.
//
//   cargo run --release --example validate_against_ndbc
//
// Environment:
//   RUN        model run as YYYYMMDDHH (default 2026092000). NDBC realtime data
//              covers the last 45 days, and ECMWF is read from the AWS mirror,
//              which keeps older runs.
//   MAX_HOUR   last forecast hour to score (default 72), every 6 hours.
//
// Both models are sampled with GridSampler: energy-weighted bilinear for Hs and
// bilinear for periods. GFS uses global.0p16, its native computational grid.
use std::{collections::HashMap, env};

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use futures_util::StreamExt;
use gribberish::{
    index::{parse_ecmwf_index, parse_noaa_index},
    message::read_messages,
};
use reqwest::Client;
use surfrs::{
    data::{ecmwf_field::ECMWFField, ecmwf_wave_point_data_record::ECMWFWavePointDataRecord},
    location::Location,
    model::{
        coalesce_ranges, ByteRange, ECMWFDataSource, ECMWFIndexQuery, ECMWFWaveModel, GFSWaveModel,
        GriddedModel, NOAADataSource,
    },
    tools::grid_sampler::{GridSampler, SampleMethod},
};

/// NDBC buoys with spectral wave data, positions from the GFS-Wave bulletins.
const BUOYS: [(&str, f64, f64); 15] = [
    ("44097", 40.97, -71.13),
    ("44025", 40.25, -73.17),
    ("44013", 42.35, -70.65),
    ("44014", 36.61, -74.84),
    ("41025", 35.01, -75.40),
    ("41002", 32.32, -75.36),
    ("41010", 28.95, -78.47),
    ("42036", 28.50, -84.52),
    ("46026", 37.75, -122.82),
    ("46042", 36.75, -122.42),
    ("46050", 44.62, -124.53),
    ("46059", 37.98, -130.00),
    ("46069", 33.65, -120.20),
    ("46086", 32.50, -118.00),
    ("51201", 21.67, -158.12),
];

/// Hs, dominant period and average period at one time.
#[derive(Clone, Copy, Default)]
struct Waves {
    hs: Option<f64>,
    tp: Option<f64>,
    tz: Option<f64>,
}

async fn get_ranges(client: &Client, url: &str, ranges: Vec<ByteRange>) -> Vec<Vec<u8>> {
    futures::stream::iter(ranges)
        .map(|range| async move {
            let response = client
                .get(url)
                .header("Range", range.http_range_header())
                .send()
                .await
                .and_then(|r| r.error_for_status());
            match response {
                Ok(r) => r.bytes().await.map(|b| b.to_vec()).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        })
        .buffer_unordered(8)
        .collect()
        .await
}

async fn ecmwf_step(
    client: &Client,
    run: DateTime<Utc>,
    hour: usize,
    locations: &[Location],
) -> Vec<Waves> {
    let model = ECMWFWaveModel::ifs_hres();
    // closest_model_run_date subtracts the publication delay, so ask for a time
    // just after this run became available.
    let query_date = Some(run + Duration::hours(9));
    let source = ECMWFDataSource::AWS;

    let Ok(index) = client
        .get(model.create_index_url(&source, hour, query_date))
        .send()
        .await
    else {
        return vec![Waves::default(); locations.len()];
    };
    let entries = parse_ecmwf_index(&index.text().await.unwrap_or_default()).unwrap_or_default();
    let query = ECMWFIndexQuery {
        fields: vec![
            ECMWFField::SignificantHeight,
            ECMWFField::PeakPeriod,
            ECMWFField::ZeroCrossingPeriod,
        ],
        ..Default::default()
    };
    let chunks = get_ranges(
        client,
        &model.create_url(&source, hour, query_date),
        query.select_coalesced(&entries),
    )
    .await;
    let messages: Vec<_> = chunks.iter().flat_map(|c| read_messages(c)).collect();

    match ECMWFWavePointDataRecord::from_messages_many(&messages, locations) {
        Ok(records) => records
            .iter()
            .map(|r| {
                r.first().map_or(Waves::default(), |r| Waves {
                    hs: r.significant_wave_height.value,
                    tp: r.peak_period.value,
                    tz: r.zero_crossing_period.value,
                })
            })
            .collect(),
        Err(_) => vec![Waves::default(); locations.len()],
    }
}

async fn gfs_step(
    client: &Client,
    run: DateTime<Utc>,
    hour: usize,
    locations: &[Location],
) -> Vec<Waves> {
    let model = GFSWaveModel::global_16();
    let query_date = Some(run + Duration::hours(6));
    let source = NOAADataSource::NODDGCP;

    let Ok(index) = client
        .get(model.create_index_url(&source, hour, query_date))
        .send()
        .await
    else {
        return vec![Waves::default(); locations.len()];
    };
    let entries =
        parse_noaa_index(&index.text().await.unwrap_or_default(), None).unwrap_or_default();
    let ranges = entries
        .iter()
        .filter(|e| {
            matches!(e.var.as_deref(), Some("HTSGW") | Some("PERPW"))
                && e.level.as_deref() == Some("surface")
        })
        .filter_map(ByteRange::from_index_entry)
        .collect();
    let chunks = get_ranges(
        client,
        &model.create_url(&source, hour, query_date),
        coalesce_ranges(ranges, 0),
    )
    .await;

    let mut waves = vec![Waves::default(); locations.len()];
    for message in chunks.iter().flat_map(|c| read_messages(c)) {
        let (Ok(abbrev), Ok(sampler)) = (
            message.variable_abbrev(),
            GridSampler::from_message(&message),
        ) else {
            continue;
        };
        let method = if abbrev == "HTSGW" {
            SampleMethod::BilinearEnergy
        } else {
            SampleMethod::Bilinear
        };
        for (w, sample) in waves.iter_mut().zip(sampler.sample_many(locations, method)) {
            match abbrev.as_str() {
                "HTSGW" => w.hs = sample.value,
                "PERPW" => w.tp = sample.value,
                _ => {}
            }
        }
    }
    waves
}

/// NDBC realtime2 observations keyed by time.
async fn observations(client: &Client, id: &str) -> Vec<(DateTime<Utc>, Waves)> {
    let url = format!("https://www.ndbc.noaa.gov/data/realtime2/{id}.txt");
    let Ok(response) = client.get(url).send().await else {
        return vec![];
    };
    let text = response.text().await.unwrap_or_default();
    let value = |s: &str| s.parse::<f64>().ok();

    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|line| {
            let c: Vec<&str> = line.split_whitespace().collect();
            if c.len() < 11 {
                return None;
            }
            let time = NaiveDateTime::parse_from_str(&c[..5].join(" "), "%Y %m %d %H %M").ok()?;
            let waves = Waves {
                hs: value(c[8]),
                tp: value(c[9]),
                tz: value(c[10]),
            };
            waves.hs.map(|_| (Utc.from_utc_datetime(&time), waves))
        })
        .collect()
}

/// The observation closest to `time`, within 30 minutes.
fn observed_at(obs: &[(DateTime<Utc>, Waves)], time: DateTime<Utc>) -> Option<Waves> {
    obs.iter()
        .filter(|(t, _)| (*t - time).num_minutes().abs() <= 30)
        .min_by_key(|(t, _)| (*t - time).num_minutes().abs())
        .map(|(_, w)| *w)
}

#[derive(Default)]
struct Score {
    errors: Vec<f64>,
}

impl Score {
    fn add(&mut self, forecast: Option<f64>, observed: Option<f64>) {
        if let (Some(f), Some(o)) = (forecast, observed) {
            self.errors.push(f - o);
        }
    }

    fn summary(&self) -> String {
        if self.errors.is_empty() {
            return format!("{:>21}", "-");
        }
        let n = self.errors.len() as f64;
        let bias = self.errors.iter().sum::<f64>() / n;
        let rmse = (self.errors.iter().map(|e| e * e).sum::<f64>() / n).sqrt();
        format!("{:>3} {:>+7.2} {:>6.2}", self.errors.len(), bias, rmse)
    }
}

#[tokio::main]
async fn main() {
    let run_text = env::var("RUN").unwrap_or_else(|_| "2026092000".into());
    let run = Utc.from_utc_datetime(
        &NaiveDateTime::parse_from_str(&format!("{run_text}00"), "%Y%m%d%H%M")
            .expect("RUN is YYYYMMDDHH"),
    );
    let max_hour: usize = env::var("MAX_HOUR")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(72);

    let client = Client::builder().user_agent("surfrs").build().unwrap();
    let locations: Vec<Location> = BUOYS
        .iter()
        .map(|(id, lat, lon)| Location::new(*lat, *lon, id.to_string()))
        .collect();

    let mut forecasts: HashMap<(&str, usize), (Waves, Waves)> = HashMap::new();
    for hour in (0..=max_hour).step_by(6) {
        let ecmwf = ecmwf_step(&client, run, hour, &locations).await;
        let gfs = gfs_step(&client, run, hour, &locations).await;
        for (i, (id, _, _)) in BUOYS.iter().enumerate() {
            forecasts.insert((id, hour), (ecmwf[i], gfs[i]));
        }
    }

    println!("Run {run}, 0-{max_hour}h every 6h. Columns: n, bias, RMSE (forecast - observed).\n");
    println!(
        "{:<6} {:>21} {:>21} {:>21} {:>21} {:>21}",
        "buoy", "IFS Hs (m)", "GFS Hs (m)", "IFS Tp (s)", "GFS Tp (s)", "IFS Tm02-APD (s)"
    );

    let mut totals: [Score; 5] = Default::default();
    for (id, _, _) in BUOYS {
        let obs = observations(&client, id).await;
        let mut scores: [Score; 5] = Default::default();
        for hour in (0..=max_hour).step_by(6) {
            let Some(o) = observed_at(&obs, run + Duration::hours(hour as i64)) else {
                continue;
            };
            let (ecmwf, gfs) = forecasts[&(id, hour)];
            for (score, total, forecast, observed) in [
                (0, 0, ecmwf.hs, o.hs),
                (1, 1, gfs.hs, o.hs),
                (2, 2, ecmwf.tp, o.tp),
                (3, 3, gfs.tp, o.tp),
                (4, 4, ecmwf.tz, o.tz),
            ] {
                scores[score].add(forecast, observed);
                totals[total].add(forecast, observed);
            }
        }
        println!(
            "{:<6} {} {} {} {} {}",
            id,
            scores[0].summary(),
            scores[1].summary(),
            scores[2].summary(),
            scores[3].summary(),
            scores[4].summary()
        );
    }
    println!(
        "{:<6} {} {} {} {} {}",
        "all",
        totals[0].summary(),
        totals[1].summary(),
        totals[2].summary(),
        totals[3].summary(),
        totals[4].summary()
    );
}
