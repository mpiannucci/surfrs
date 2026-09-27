use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct NOAADataSourceError(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum NOAADataSource {
    NODDAWS,
    NODDGCP,
    NOMADS,
}

impl TryFrom<&str> for NOAADataSource {
    type Error = NOAADataSourceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let lowered = value.to_lowercase();

        if lowered.contains("aws") || lowered.contains("amazon") {
            Ok(NOAADataSource::NODDAWS)
        } else if lowered.contains("gcp") || lowered.contains("gcs") || lowered.contains("google") {
            Ok(NOAADataSource::NODDGCP)
        } else if lowered.contains("nomads") || lowered.contains("noaa") {
            Ok(NOAADataSource::NOMADS)
        } else {
            Err(NOAADataSourceError(format!("Unknown data source: {value}")))
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
    SixHourly,
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
            ModelTimeOutputResolution::SixHourly => index * 6,
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
            ModelTimeOutputResolution::SixHourly => hour / 6,
        }
    }

    pub fn indexes_for_hour_range(&self, start_hour: usize, end_hour: usize) -> Vec<usize> {
        (start_hour..=end_hour)
            .map(|h| self.index_for_hour(h))
            .collect()
    }

    pub fn hours_for_index_range(&self, start_index: usize, end_index: usize) -> Vec<usize> {
        (start_index..=end_index)
            .map(|h| self.hour_for_index(h))
            .collect()
    }

    pub fn hours_for_hour_range(&self, start_hour: usize, end_hour: usize) -> Vec<usize> {
        let start_index = self.index_for_hour(start_hour);
        let end_index = self.index_for_hour(end_hour);
        self.hours_for_index_range(start_index, end_index)
    }
}

pub trait GriddedModel {
    /// The mirrors this model family is published on.
    type Source;

    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    /// The most recent model run expected to be fully published at `date`.
    fn closest_model_run_date(&self, date: &DateTime<Utc>) -> DateTime<Utc>;

    /// The output hours to pass to `create_url` for the run starting at `run`.
    fn forecast_hours(&self, run: &DateTime<Utc>) -> Vec<usize>;

    fn url_root(&self, source: &Self::Source) -> &'static str;

    fn create_url(
        &self,
        source: &Self::Source,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String;

    fn create_index_url(
        &self,
        source: &Self::Source,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String {
        format!("{}.idx", self.create_url(source, output_hour, query_date))
    }
}

#[cfg(test)]
mod test {
    use crate::model::NOAADataSource;

    use super::ModelTimeOutputResolution;

    #[test]
    fn test_model_source_parse() {
        assert_eq!(
            NOAADataSource::try_from("NOMADS").unwrap(),
            NOAADataSource::NOMADS
        );
        assert_eq!(
            NOAADataSource::try_from("noaa").unwrap(),
            NOAADataSource::NOMADS
        );
        assert_eq!(
            NOAADataSource::try_from("NODDAWS").unwrap(),
            NOAADataSource::NODDAWS
        );
        assert_eq!(
            NOAADataSource::try_from("noddgcp").unwrap(),
            NOAADataSource::NODDGCP
        );
        assert_eq!(
            NOAADataSource::try_from("noddgcs").unwrap(),
            NOAADataSource::NODDGCP
        );
        assert!(NOAADataSource::try_from("unknown").is_err());
    }

    #[test]
    fn test_model_output_time_index_to_hour() {
        assert_eq!(ModelTimeOutputResolution::Hourly.hour_for_index(140), 140);
        assert_eq!(
            ModelTimeOutputResolution::ThreeHourly.hour_for_index(20),
            60
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hour_for_index(90),
            90
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hour_for_index(130),
            150
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).hour_for_index(18),
            54
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).hour_for_index(104),
            384
        );
        assert_eq!(ModelTimeOutputResolution::SixHourly.hour_for_index(60), 360);
    }

    #[test]
    fn test_model_output_time_hour_to_index() {
        assert_eq!(ModelTimeOutputResolution::Hourly.index_for_hour(140), 140);
        assert_eq!(
            ModelTimeOutputResolution::ThreeHourly.index_for_hour(63),
            21
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridHourlyThreeHourly(120).index_for_hour(90),
            90
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridHourlyThreeHourly(120).index_for_hour(132),
            124
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).index_for_hour(240),
            80
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(240).index_for_hour(384),
            104
        );
        assert_eq!(ModelTimeOutputResolution::SixHourly.index_for_hour(360), 60);
    }

    #[test]
    fn test_model_output_time_hours_for_hour_range() {
        let hourly_hours = [117, 118, 119, 120, 121, 122, 123, 124, 125, 126];
        let hybrid_hourly = [117, 118, 119, 120, 123, 126];
        let three_hourly_hours = [117, 120, 123, 126];
        let hybrid_three_hourly = [234, 237, 240, 246, 252, 258];

        assert_eq!(
            ModelTimeOutputResolution::Hourly.hours_for_hour_range(117, 126),
            hourly_hours
        );
        assert_eq!(
            ModelTimeOutputResolution::ThreeHourly.hours_for_hour_range(117, 126),
            three_hourly_hours
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridHourlyThreeHourly(120).hours_for_hour_range(117, 126),
            hybrid_hourly
        );
        assert_eq!(
            ModelTimeOutputResolution::HybridThreeHourlySixHourly(240)
                .hours_for_hour_range(234, 258),
            hybrid_three_hourly
        );
    }
}
