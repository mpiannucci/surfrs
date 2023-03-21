use std::{future, fs};

use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use gribberish::message::{read_messages};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use surfrs::{location::Location, model::{GFSWaveModel, ModelDataSource, NOAAModel}, data::gfs_wave_grib_point_data_record::GFSWaveGribPointDataRecord, units::{UnitConvertible, UnitSystem, Direction, Unit}, dimensional_data::DimensionalData, swell::Swell, tools::{waves::{break_wave, estimate_breaking_wave_height}, vector::min_max}};


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
        self.swell_components.iter_mut().for_each(|c| c.to_units(new_units));
        self.minimum_breaking_height.to_units(new_units);
        self.maximum_breaking_height.to_units(new_units);
    }
}

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

    let data = try_join_all(requests)
        .await
        .unwrap()
        .iter()
        .map(|b| {
            // Extract data to grib data records
            let messages = read_messages(b).collect();
            let record = GFSWaveGribPointDataRecord::from_messages(&atlantic_wave_model, &messages, &location);
            
            // Compute breaking wave data
            let breaking_wave_heights = record.swell_components
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
                unit: Unit::Meters 
            };

            // For now assume this is significant wave height as the max and the rms as the min
            let min_breaking_wave_height = DimensionalData { 
                value: Some(breaking_wave_height / 1.4), 
                variable_name: "min breaking wave height".into(), 
                unit: Unit::Meters 
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

    // Combine and export json forecast data

    let data = serde_json::to_string(&data).unwrap();
    fs::write("forecast.json", data).unwrap();
}