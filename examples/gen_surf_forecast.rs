use std::time::Instant;
use std::{fs};

use rayon::prelude::*;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use gribberish::message::read_messages;
use rayon::prelude::IntoParallelRefIterator;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use surfrs::{
    data::{
        gfs_wave_grib_point_data_record::GFSWaveGribPointDataRecord,
        nws_weather_forecast_data_record::{NwsGridPointData, NwsWeatherForecastDataRecordCollection},
    },
    dimensional_data::DimensionalData,
    location::Location,
    model::{GFSWaveModel, ModelDataSource, NOAAModel},
    swell::Swell,
    tools::{
        vector::min_max,
        waves::{estimate_breaking_wave_height},
    },
    units::{Direction, Unit, UnitConvertible, UnitSystem},
    weather::{create_hourly_forecast_url, create_points_url},
};

#[derive(Serialize, Deserialize)]
struct SurfForecastDataRecord {
    pub date: DateTime<Utc>,
    pub wave_summary: Swell,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub swell_components: Vec<Swell>,
    pub minimum_breaking_height: DimensionalData<f64>,
    pub maximum_breaking_height: DimensionalData<f64>,
}

impl UnitConvertible<SurfForecastDataRecord> for SurfForecastDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) {
        self.wind_speed.to_units(new_units);
        self.wave_summary.to_units(new_units);
        self.swell_components
            .iter_mut()
            .for_each(|c| c.to_units(new_units));
        self.minimum_breaking_height.to_units(new_units);
        self.maximum_breaking_height.to_units(new_units);
    }
}

#[tokio::main]
async fn main() {
    let start = Instant::now();

    let location = Location::new(41.35, -71.4, "Block Island Sound".into());
    let depth = 30.0;
    let angle = 145.0;
    let slope = 0.02;
    let now = chrono::Utc::now();

    println!("Fetching GFS Wave Model Data");

    let atlantic_wave_model = GFSWaveModel::atlantic();

    let client = Client::new();
    let requests = (0..120).map(|i| {
        let url = atlantic_wave_model.create_url(&ModelDataSource::NODDAWS, i, Some(now));
        let client = &client;
        async move {
            let resp = client.get(url).send().await?;
            resp.bytes().await
        }
    });

    let mut wave_data = try_join_all(requests)
        .await
        .unwrap()
        .par_iter()
        .enumerate()
        .map(|(i, b)| {
            if i == 0 {
                println!("Processing GFS Wave Model Data");
            }

            // Extract data to grib data records
            let messages = read_messages(b).collect();
            let record = GFSWaveGribPointDataRecord::from_messages(
                &atlantic_wave_model,
                &messages,
                &location,
                0.167
            ).unwrap();

            // Compute breaking wave data
            let breaking_wave_heights = record
                .swell_components
                .iter()
                .filter_map(|s| estimate_breaking_wave_height(s, angle, slope, depth).ok())
                .collect::<Vec<_>>();

            // https://github.com/mpiannucci/surfpy/blob/af65f70c36c37b3454305711058cabc15d129028/surfpy/swell.py#L42
            let (_, breaking_wave_height) = min_max(&breaking_wave_heights);

            // Take the maximum breaking height and give it a scale factor of 0.9 for refraction
            // or anything we are not checking for.
            let breaking_wave_height = breaking_wave_height * 0.8;
            let max_breaking_wave_height = DimensionalData {
                value: Some(breaking_wave_height),
                variable_name: "max reaking wave height".into(),
                unit: Unit::Meters,
            };

            // For now assume this is significant wave height as the max and the rms as the min
            let min_breaking_wave_height = DimensionalData {
                value: Some(breaking_wave_height / 1.4),
                variable_name: "min breaking wave height".into(),
                unit: Unit::Meters,
            };

            let mut record = SurfForecastDataRecord {
                date: record.date,
                wave_summary: record.wave_summary,
                wind_speed: record.wind_speed,
                wind_direction: record.wind_direction,
                swell_components: record.swell_components,
                minimum_breaking_height: min_breaking_wave_height,
                maximum_breaking_height: max_breaking_wave_height,
            };

            record.to_units(&UnitSystem::English);

            record
        })
        .collect::<Vec<_>>();

    // Fetch weather forecast
    println!("Fetching NWS Hourly Weather Forecast");

    let client = reqwest::Client::builder()
        .user_agent("hopewaves.app")
        .build()
        .unwrap();

    let weather_location = Location::new(41.41, -71.45, "Narragansett Pier".into());
    let weather_url = create_points_url(&weather_location);

    let weather_gridpoints = client
        .get(&weather_url)
        .send()
        .await
        .unwrap()
        .json::<NwsGridPointData>()
        .await
        .unwrap();

    let weather_url = create_hourly_forecast_url(
        &weather_gridpoints.properties.grid_id,
        &weather_gridpoints.properties.grid_x,
        &weather_gridpoints.properties.grid_y,
    );
    let weather_forecast = client
        .get(&weather_url)
        .send()
        .await
        .unwrap()
        .json::<NwsWeatherForecastDataRecordCollection>()
        .await
        .unwrap()
        .records();

    println!("Merging Weather and Wave Data");

    // Combine and export json forecast data
    for wave_record in wave_data.iter_mut() {
        let Some(weather_record) = weather_forecast
            .par_iter()
            .find_any(|wx| wx.start_time == wave_record.date) else {
                continue;
        };

        wave_record.wind_speed = weather_record.wind_speed.clone();
        wave_record.wind_direction = weather_record.wind_direction.clone();
    }

    println!("Writing Surf Forecast Data");
    let data = serde_json::to_string(&wave_data).unwrap();
    fs::write("gfs_wave_forecast_nws_wind.json", data).unwrap();

    println!(
        "Finished Surf Forecast Generation in {} seconds",
        start.elapsed().as_secs()
    );
}
