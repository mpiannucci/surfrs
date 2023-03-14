use std::future;

use futures_util::future::try_join_all;
use reqwest::Client;
use surfrs::{location::Location, model::{GFSWaveModel, ModelDataSource, NOAAModel}};

#[tokio::main]
async fn main() {
    let location = Location::new(41.35, -71.4, "Block Island Sound".into());
    let depth = 30.0;
    let angle =  145.0;
    let slope = 0.02;
    let now = chrono::Utc::now();

    println!("Fetching GFS Wave Model Data");

    let atlantic_wave_model = GFSWaveModel::atlantic();

    let client = Client::new();
    let requests = (0..120)
        .map(|i| {
            let url = atlantic_wave_model.create_url(&ModelDataSource::NODDAWS, i, Some(now));
            let client = &client;
            async move {
                let resp = client.get(url).send().await?;
                resp.bytes().await
            }
        });

    let data = try_join_all(requests).await.unwrap();

    // Extract data to grib data records

    // Compute breaking wave data

    // Fetch weather forecast

    // Combine and export json forecast data
}