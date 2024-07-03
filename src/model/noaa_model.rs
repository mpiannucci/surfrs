use chrono::{DateTime, Utc};
use geojson::Feature;
use gribberish::{error::GribberishError, message::Message};
use serde::{Deserialize, Serialize};

use crate::{
    location::{normalize_latitude, normalize_longitude, Location},
    tools::{
        contour::compute_latlng_gridded_contours, analysis::{bilerp, lerp},
    }, units::{UnitSystem, Unit}
};

#[derive(Debug, Clone)]
pub struct ModelDataSourceError(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ModelDataSource {
    NODDAWS,
    NODDGCP,
    NOMADS,
}

impl TryFrom<&str> for ModelDataSource {
    type Error = ModelDataSourceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let lowered = value.to_lowercase();

        if lowered.contains("aws") || lowered.contains("amazon") {
            Ok(ModelDataSource::NODDAWS)
        } else if lowered.contains("gcp") || lowered.contains("gcs") || lowered.contains("google") {
            Ok(ModelDataSource::NODDGCP)
        } else if lowered.contains("nomads") || lowered.contains("noaa") {
            Ok(ModelDataSource::NOMADS)
        } else {
            Err(ModelDataSourceError(format!("Unknown data source: {value}")))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTimeOutputResolution {
    Hourly,
    HybridHourlyThreeHourly(usize),
    ThreeHourly,
    HybridThreeHourlySixHourly(usize),
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
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(cutoff) => {
                if (index * 3) <= *cutoff {
                    index * 3
                } else {
                    cutoff + ((index * 3 - cutoff) / 3) * 6
                }
            }
        }
    }

    pub fn index_for_hour(&self, hour: usize) -> usize {
        match self {
            ModelTimeOutputResolution::Hourly => hour,
            ModelTimeOutputResolution::HybridHourlyThreeHourly(cutoff) => {
                if hour <= *cutoff {
                    hour
                } else {
                    cutoff + (hour - cutoff) / 3
                }
            }
            ModelTimeOutputResolution::ThreeHourly => hour / 3,
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(cutoff) => {
                if hour <= *cutoff {
                    hour / 3
                } else {
                    cutoff / 3 + (hour - cutoff) / 6
                }
            }
        }
    }

    pub fn indexes_for_hour_range(&self, start_hour: usize, end_hour: usize) -> Vec<usize> {
        (start_hour..=end_hour).map(|h| self.index_for_hour(h)).collect()
    }

    pub fn hours_for_index_range(&self, start_index: usize, end_index: usize) -> Vec<usize> {
        (start_index..=end_index).map(|h| self.hour_for_index(h)).collect()
    }

    pub fn hours_for_hour_range(&self, start_hour: usize, end_hour: usize) -> Vec<usize> {
        let start_index = self.index_for_hour(start_hour);
        let end_index = self.index_for_hour(end_hour);
        self.hours_for_index_range(start_index, end_index)
    }
}

pub trait NOAAModel {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn time_resolution(&self) -> ModelTimeOutputResolution;
    fn closest_model_run_date(&self, date: &DateTime<Utc>) -> DateTime<Utc>;
    fn url_root(&self, source: &ModelDataSource) -> &'static str;
    fn hour_for_index(&self, index: usize) -> usize {
        self.time_resolution().hour_for_index(index)
    }
    fn index_for_hour(&self, hour: usize) -> usize {
        self.time_resolution().index_for_hour(hour)
    }

    fn create_url(
        &self,
        source: &ModelDataSource,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String;

    fn create_idx_url(
        &self,
        source: &ModelDataSource,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String {
        format!("{}.idx", self.create_url(source, output_hour, query_date))
    }

    fn query_location_tolerance(&self, location: &Location, tolerance: &f64, message: &Message) -> Result<Vec<f64>, GribberishError> {
        let projector = message.latlng_projector()?;
        let bbox = projector.bbox();

        if !location.within_bbox(&bbox) {
            return Err(GribberishError::MessageError("location is not within the models bounds".into()));
        }

        let min_lat = location.relative_latitude() - *tolerance;
        let max_lat = location.relative_latitude() + *tolerance;
        let min_lng = location.relative_longitude() - *tolerance;
        let max_lng = location.relative_longitude() + *tolerance;

        let data = message.data()?;

        let (lat, lng) = projector.lat_lng();

        let data = data
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.is_nan())
            .filter(|(i, _)| {
                let row = i / lng.len();
                let col = i % lng.len();

                normalize_latitude(lat[row]) >= min_lat && normalize_latitude(lat[row]) <= max_lat && normalize_longitude(lng[col]) >= min_lng && normalize_longitude(lng[col]) <= max_lng
            })
            .map(|(_, v)| *v)
            .collect();

        Ok(data)
    }

    fn query_location_data(&self, location: &Location, message: &Message) -> Result<f64, GribberishError> {
        let projector = message.latlng_projector()?;
        let bbox = projector.bbox();

        if !location.within_bbox(&bbox) {
            return Err(GribberishError::MessageError("location is not within the models bounds".into()));
        }

        // This only works for regular grids.
        let (lat_size, lng_size) = message.grid_dimensions()?;
        let start = projector.latlng_start();
        let end = projector.latlng_end();

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

    fn interp_location_data(&self, location: &Location, message: &Message) -> Result<f64, GribberishError> {
        let projector = message.latlng_projector()?;
        let bbox = projector.bbox();

        if !location.within_bbox(&bbox) {
            return Err(GribberishError::MessageError("location is not within the models bounds".into()));
        }

        // This only works for regular grids.
        let (lat_size, lng_size) = message.grid_dimensions()?;
        let start = projector.latlng_start();
        let end = projector.latlng_end();

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

        let (lat, lng) = projector.lat_lng();
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
    ) -> Result<Vec<Feature>, GribberishError> {
        // This only works for regular grids.
        let projector = message.latlng_projector()?;
        let (lat_count, lng_count) = message.grid_dimensions()?;

        let (lat_start, lng_start) = projector.latlng_start();
        let (lat_end, lng_end) = projector.latlng_end();

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
        .map_err(|_| GribberishError::MessageError("Failed to contour data".into()))
    }
}

#[cfg(test)]
mod test {
    use crate::model::ModelDataSource;

    use super::ModelTimeOutputResolution;

    #[test]
    fn test_model_source_parse() {
        assert_eq!(ModelDataSource::try_from("NOMADS").unwrap(), ModelDataSource::NOMADS);
        assert_eq!(ModelDataSource::try_from("noaa").unwrap(), ModelDataSource::NOMADS);
        assert_eq!(ModelDataSource::try_from("NODDAWS").unwrap(), ModelDataSource::NODDAWS);
        assert_eq!(ModelDataSource::try_from("noddgcp").unwrap(), ModelDataSource::NODDGCP);
        assert_eq!(ModelDataSource::try_from("noddgcs").unwrap(), ModelDataSource::NODDGCP);
        assert!(ModelDataSource::try_from("unknown").is_err());
    }

    #[test]
    fn test_model_output_time_index_to_hour() {
        assert_eq!(ModelTimeOutputResolution::Hourly.hour_for_index(140), 140);
        assert_eq!(ModelTimeOutputResolution::ThreeHourly.hour_for_index(20), 60);
        assert_eq!(ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hour_for_index(90), 90);
        assert_eq!(ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hour_for_index(130), 150);
        assert_eq!(ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).hour_for_index(18), 54);
        assert_eq!(ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).hour_for_index(104), 384);
    }

    #[test]
    fn test_model_output_time_hour_to_index() {
        assert_eq!(ModelTimeOutputResolution::Hourly.index_for_hour(140), 140);
        assert_eq!(ModelTimeOutputResolution::ThreeHourly.index_for_hour(63), 21);
        assert_eq!(ModelTimeOutputResolution::HybridHourlyThreeHourly(120).index_for_hour(90), 90);
        assert_eq!(ModelTimeOutputResolution::HybridHourlyThreeHourly(120).index_for_hour(132), 124);
        assert_eq!(ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).index_for_hour(240), 80);
        assert_eq!(ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).index_for_hour(384), 104);
    }

    #[test]
    fn test_model_output_time_hours_for_hour_range() {
        let hourly_hours = [117, 118, 119, 120, 121, 122, 123, 124, 125, 126];
        let hybrid_hourly = [117, 118, 119, 120, 123, 126];
        let three_hourly_hours = [117, 120, 123, 126];
        let hybrid_three_hourly = [234, 237, 240, 246, 252, 258];

        assert_eq!(ModelTimeOutputResolution::Hourly.hours_for_hour_range(117, 126), hourly_hours);
        assert_eq!(ModelTimeOutputResolution::ThreeHourly.hours_for_hour_range(117, 126), three_hourly_hours);
        assert_eq!(ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hours_for_hour_range(117, 126), hybrid_hourly);
        assert_eq!(ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).hours_for_hour_range(234, 258), hybrid_three_hourly);
    }
}
