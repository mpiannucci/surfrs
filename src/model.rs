use chrono::{DateTime, Datelike, Timelike, Utc};
use geojson::Feature;
use gribberish::message::Message;
use itertools::Either;
use serde::{Deserialize, Serialize};

use crate::{
    location::{normalize_latitude, normalize_longitude, Location},
    tools::{
        contour::compute_contours, date::closest_gfs_model_datetime, linspace::linspace,
        vector::min_max_fill,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ModelDataSource {
    NODDAWS,
    NODDGCP,
    NOMADS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTimeOutputResolution {
    Hourly,
    HybridHourlyThreeHourly(usize),
    ThreeHourly,
}

impl ModelTimeOutputResolution {
    pub fn hour_for_index(&self, index: usize) -> usize {
        match self {
            ModelTimeOutputResolution::Hourly => index,
            ModelTimeOutputResolution::HybridHourlyThreeHourly(cutoff) => {
                if index <= *cutoff {
                    index
                } else {
                    cutoff + (index - cutoff) * 3
                }
            }
            ModelTimeOutputResolution::ThreeHourly => index * 3,
        }
    }
}

pub trait NOAAModel {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn time_resolution(&self) -> ModelTimeOutputResolution;
    fn closest_model_run_date(&self, date: &DateTime<Utc>) -> DateTime<Utc>;
    fn url_root(&self, source: &ModelDataSource) -> &'static str;
    fn create_url(
        &self,
        source: &ModelDataSource,
        output_index: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String;

    fn query_location_data(&self, location: &Location, message: &Message) -> Result<f64, String> {
        let bbox = message.location_bbox()?;

        if !location.within_bbox(&bbox) {
            return Err("location is not within the models bounds".into());
        }

        // This only works for regular grids.
        let (lat_size, lng_size) = message.grid_dimensions()?;
        let (start, end) = message.grid_bounds()?;

        let lng_step = (end.1 - start.1) / lng_size as f64;
        let lat_step = (end.0 - start.0) / lat_size as f64;

        let lng_index = ((location.relative_longitude() - normalize_longitude(start.1)) / lng_step)
            .abs()
            .round() as usize;
        let lat_index = ((location.relative_latitude() - normalize_latitude(start.0)) / lat_step)
            .abs()
            .round() as usize;

        let data = message.data()?;
        let latlng_index = lng_index + lng_size * lat_index;
        let value = data[latlng_index];
        Ok(value)
    }

    fn contour_data(
        &self,
        message: &Message,
        threshold_min: Option<f64>,
        threshold_max: Option<f64>,
        threshold_count: Option<usize>,
    ) -> Result<Vec<Feature>, String> {
        // This only works for regular grids.
        let (lat_size, mut lng_size) = message.grid_dimensions()?;
        let mut data_count = lat_size * lng_size;
        let (start, mut end) = message.grid_bounds()?;

        let lng_step = (end.1 - start.1) / lng_size as f64;
        let diff = normalize_longitude(end.1) - normalize_longitude(start.1);

        let mut data = if diff.abs() - lng_step.abs() < 0.001 {
            let data = message.data()?;
            let data = data
                .iter()
                .enumerate()
                .flat_map(|(i, v)| {
                    let lng_index = i % lng_size;
                    if lng_index == lng_size - 1 {
                        let lat_index = i / lng_size;
                        Either::Left([*v, *(&data[lat_index * lng_size])].into_iter())
                    } else {
                        Either::Right(std::iter::once(*v))
                    }
                })
                .collect(); 
            end.1 = (end.1 + lng_step).ceil();
            lng_size += 1;
            data_count = lat_size * lng_size;
            data
        } else {
            message.data()?
        };

        let (min, max) = min_max_fill(&mut data, -99999.0);
        let thresholds = linspace(
            threshold_min.unwrap_or(min),
            threshold_max.unwrap_or(max),
            threshold_count.unwrap_or(20),
        )
        .collect::<Vec<_>>();

        compute_contours(
            &mut data[0..data_count],
            lng_size,
            lat_size,
            &thresholds,
            Some(|point: &Vec<f64>| {
                let x = start.1 + (end.1 - start.1) * (point[0] / (lng_size as f64));
                let y = start.0 + (end.0 - start.0) * (point[1] / (lat_size as f64));
                vec![x, y]
            }),
        )
        .map_err(|_| "Failed to contour data".into())
    }
}

pub struct GFSWaveModel {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

impl GFSWaveModel {
    pub fn atlantic() -> Self {
        GFSWaveModel {
            id: "atlocn.0p16",
            name: "GFS Wave Atlantic",
            description: "GFS Wave Model: Atlantic 0.16 degree",
        }
    }

    pub fn west_coast() -> Self {
        GFSWaveModel {
            id: "wcoast.0p16",
            name: "GFS Wave West Coast US",
            description: "GFS Wave Model: US West Coast 0.16 degree",
        }
    }

    pub fn east_pacific() -> Self {
        GFSWaveModel {
            id: "epacif.0p16",
            name: "GFS Wave East Pacific",
            description: "GFS Wave Model: East Pacific 0.16 degree",
        }
    }

    pub fn global_16() -> Self {
        GFSWaveModel {
            id: "global.0p16",
            name: "GFS Wave Global",
            description: "GFS Wave Model: Global 0.16 degree",
        }
    }

    pub fn global_25() -> Self {
        GFSWaveModel {
            id: "global.0p25",
            name: "GFS Wave Global",
            description: "GFS Wave Model: Global 0.25 degree",
        }
    }
}

impl NOAAModel for GFSWaveModel {
    fn id(&self) -> &'static str {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn closest_model_run_date(&self, date: &DateTime<Utc>) -> DateTime<Utc> {
        closest_gfs_model_datetime(date)
    }

    fn time_resolution(&self) -> ModelTimeOutputResolution {
        ModelTimeOutputResolution::HybridHourlyThreeHourly(120)
    }

    fn url_root(&self, source: &ModelDataSource) -> &'static str {
        match source {
            ModelDataSource::NODDAWS => "https://noaa-gfs-bdp-pds.s3.amazonaws.com",
            ModelDataSource::NOMADS => "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod",
            ModelDataSource::NODDGCP => "https://storage.googleapis.com/global-forecast-system",
        }
    }

    fn create_url(
        &self,
        source: &ModelDataSource,
        output_index: usize,
        model_date: Option<DateTime<Utc>>,
    ) -> String {
        let id = self.id();
        let base = self.url_root(source);
        let model_date = self.closest_model_run_date(&model_date.unwrap_or(Utc::now()));
        let timestep = self.time_resolution().hour_for_index(output_index);
        let year = model_date.year();
        let month = model_date.month();
        let day = model_date.day();
        let hour = model_date.hour();

        format!(
            "{base}/gfs.{year}{month:02}{day:02}/{hour:02}/wave/gridded/gfswave.t{hour:02}z.{id}.f{timestep}.grib2"
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use crate::model::GFSWaveModel;

    use super::{ModelDataSource, NOAAModel};

    #[test]
    fn test_gfs_wave_url() {
        let truth = "https://storage.googleapis.com/global-forecast-system/gfs.20230117/06/wave/gridded/gfswave.t06z.atlocn.0p16.f115.grib2";

        let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 01, 17, 13, 0, 0).unwrap();

        let gfs_wave = GFSWaveModel::atlantic();
        let url = gfs_wave.create_url(&ModelDataSource::NODDGCP, 115, Some(date));
        assert_eq!(url, truth);

        let truth = "https://storage.googleapis.com/global-forecast-system/gfs.20230117/06/wave/gridded/gfswave.t06z.atlocn.0p16.f126.grib2";
        let url = gfs_wave.create_url(&ModelDataSource::NODDGCP, 122, Some(date));
        assert_eq!(url, truth);
    }
}
