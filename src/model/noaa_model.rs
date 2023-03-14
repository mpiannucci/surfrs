use chrono::{DateTime, Utc};
use geojson::Feature;
use gribberish::message::Message;
use serde::{Deserialize, Serialize};

use crate::{
    location::{normalize_latitude, normalize_longitude, Location},
    tools::{
        contour::{compute_latlng_gridded_contours}, analysis::{bilerp, lerp},
    }, units::{UnitSystem, Unit}
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

    fn interp_location_data(&self, location: &Location, message: &Message) -> Result<f64, String> {
        let bbox = message.location_bbox()?;

        if !location.within_bbox(&bbox) {
            return Err("location is not within the models bounds".into());
        }

        // This only works for regular grids.
        let (lat_size, lng_size) = message.grid_dimensions()?;
        let (start, end) = message.grid_bounds()?;

        let lng_step = (end.1 - start.1) / lng_size as f64;
        let lat_step = (end.0 - start.0) / lat_size as f64;

        let lng_lower_index = ((location.relative_longitude() - normalize_longitude(start.1)) / lng_step)
            .abs()
            .floor() as usize;
        let lng_upper_index = ((location.relative_longitude() - normalize_longitude(start.1)) / lng_step)
            .abs()
            .ceil() as usize;
        let lat_lower_index = ((location.relative_latitude() - normalize_latitude(start.0)) / lat_step)
            .abs()
            .floor() as usize;
        let lat_upper_index = ((location.relative_latitude() - normalize_latitude(start.0)) / lat_step)
            .abs()
            .ceil() as usize;

        let (lat, lng) = message.latitude_longitude_arrays()?;
        let data = message.data()?;

        let a = data[lat_lower_index * lng_size + lng_lower_index];
        let b = data[lat_lower_index * lng_size + lng_upper_index];
        let c = data[lat_upper_index * lng_size + lng_lower_index];
        let d = data[lat_upper_index * lng_size + lng_upper_index];

        let x0 = normalize_longitude(lng[lng_lower_index]);
        let x1 = normalize_longitude(lng[lng_upper_index]);
        let y0 = normalize_latitude(lat[lat_lower_index * lng_size]);
        let y1 = normalize_latitude(lat[lat_upper_index * lng_size]);

        let value = if lat_lower_index == lat_upper_index && lng_lower_index == lng_upper_index {
            a
        } else if lat_lower_index == lat_upper_index {
            lerp(&a, &b, &location.longitude, &x0, &x1)
        } else if lng_lower_index == lat_lower_index {
            lerp(&a, &c, &location.latitude, &y0, &y1)
        } else {
            bilerp(&a, &b, &c, &d, &location.longitude, &x0, &x1, &location.latitude, &y0, &y1)
        };

        Ok(value)
    }

    fn contour_data(
        &self,
        message: &Message,
        threshold_min: Option<f64>,
        threshold_max: Option<f64>,
        threshold_count: Option<usize>,
        units: Option<UnitSystem>,
    ) -> Result<Vec<Feature>, String> {
        // This only works for regular grids.
        let (lat_count, lng_count) = message.grid_dimensions()?;
        let ((lat_start, lng_start), (lat_end, lng_end)) = message.grid_bounds()?;

        let mut unit_abbrev = message.unit()?;
        let data = if let Some(unit_system) = units.as_ref() {
            let unit = Unit::from(unit_abbrev.as_str());
            let target = unit.convert_system(unit_system);
            unit_abbrev = target.abbreviation().into();

            let data = message.data()?;
            if unit != target {
                data
                    .into_iter()
                    .map(|v| unit.convert(v, &target))
                    .collect()
            } else {
                data
            }
        } else {
            message.data()?
        };

        compute_latlng_gridded_contours(
            data, 
            lng_count, 
            lat_count, 
            lng_start, 
            lng_end, 
            lat_start, 
            lat_end, 
            threshold_min, 
            threshold_max, 
            threshold_count, 
            Some(|index: &usize, value: &f64| {
                if index % 2 > 0 {
                    format!(
                        "{:.0}{}",
                        value.round(),
                        unit_abbrev
                    )
                } else {
                    "".to_string()
                }
            })
        )
        .map_err(|_| "Failed to contour data".into())
    }
}
