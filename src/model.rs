use std::collections::HashSet;

use chrono::{DateTime, Datelike, Timelike, Utc};
use gribberish::message::Message;
use serde::{Deserialize, Serialize};

use crate::{
    location::{normalize_latitude, normalize_longitude, Location},
    tools::date::closest_gfs_model_datetime,
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

        // GFS Wave grids are all regular, so we can just get the four neighbors
        let (lat_size, lng_size) = message.grid_dimensions()?;

        let lng_step = (bbox.2 - bbox.0).abs() / lng_size as f64;
        let lat_step = (bbox.3 - bbox.1).abs() / lat_size as f64;

        let lng_index = (location.relative_longitude() - normalize_longitude(bbox.0)) / lng_step;
        let lat_index = (location.relative_latitude() - normalize_latitude(bbox.1)) / lat_step;

        let floored_lng = lng_index.floor() as usize;
        let ceiled_lng = lng_index.ceil() as usize;
        let floored_lat = lat_index.floor() as usize;
        let ceiled_lat = lat_index.ceil() as usize;
        
        let mut indexes = HashSet::new();
        indexes.insert(floored_lng + (floored_lat * lng_size));
        indexes.insert(ceiled_lng + (floored_lat * lng_size));
        indexes.insert(floored_lng + (ceiled_lat * lng_size));
        indexes.insert(ceiled_lng + (ceiled_lat * lng_size));

        let avg_count = indexes.len();
        let data = message.data()?;

        // TODO: THis is just simple average, should do real interpolation eventually
        let sum = indexes.iter().fold(0.0, |acc, i| {
            acc + data[*i]
        });
        let value = sum / avg_count as f64;

        Ok(value)
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
