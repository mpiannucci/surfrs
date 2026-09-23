use chrono::prelude::*;

use crate::tools::date::closest_gfs_model_gridded_datetime;

use super::{GriddedModel, ModelTimeOutputResolution, NOAADataSource};

pub struct GEFSWaveModel {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

impl GEFSWaveModel {
    pub fn global_25_mean() -> Self {
        GEFSWaveModel {
            id: "mean.global.0p25",
            name: "GEFS Wave Global",
            description: "GEFS Wave Model: Global 0.25 degree Ensemble Mean",
        }
    }

    pub fn global_25_spread() -> Self {
        GEFSWaveModel {
            id: "spread.global.0p25",
            name: "GEFS Wave Global",
            description: "GEFS Wave Model: Global 0.25 degree Ensemble Spread",
        }
    }

    pub fn time_resolution(&self) -> ModelTimeOutputResolution {
        ModelTimeOutputResolution::HybridThreeHourlySixHourly(240)
    }
}

impl GriddedModel for GEFSWaveModel {
    type Source = NOAADataSource;

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
        closest_gfs_model_gridded_datetime(date)
    }

    fn forecast_hours(&self, _run: &DateTime<Utc>) -> Vec<usize> {
        self.time_resolution().hours_for_hour_range(0, 384)
    }

    fn url_root(&self, source: &NOAADataSource) -> &'static str {
        match source {
            NOAADataSource::NODDAWS => "https://noaa-gefs-pds.s3.amazonaws.com",
            NOAADataSource::NOMADS => "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gefs/prod",
            NOAADataSource::NODDGCP => "",
        }
    }

    fn create_url(
        &self,
        source: &NOAADataSource,
        output_index: usize,
        model_date: Option<DateTime<Utc>>,
    ) -> String {
        let id = self.id();
        let base = self.url_root(source);
        let model_date = self.closest_model_run_date(&model_date.unwrap_or(Utc::now()));
        let year = model_date.year();
        let month = model_date.month();
        let day = model_date.day();
        let hour = model_date.hour();

        format!(
            "{base}/gefs.{year}{month:02}{day:02}/{hour:02}/wave/gridded/gefs.wave.t{hour:02}z.{id}.f{output_index:03}.grib2"
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use super::{GEFSWaveModel, GriddedModel, NOAADataSource};

    #[test]
    fn test_gefs_wave_url() {
        let truth = "https://noaa-gefs-pds.s3.amazonaws.com/gefs.20230525/06/wave/gridded/gefs.wave.t06z.spread.global.0p25.f216.grib2";

        let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 05, 25, 13, 0, 0).unwrap();

        let gefs_wave = GEFSWaveModel::global_25_spread();
        let url = gefs_wave.create_url(&NOAADataSource::NODDAWS, 216, Some(date));
        assert_eq!(url, truth);

        let truth = "https://noaa-gefs-pds.s3.amazonaws.com/gefs.20230525/06/wave/gridded/gefs.wave.t06z.spread.global.0p25.f294.grib2";
        let url = gefs_wave.create_url(&NOAADataSource::NODDAWS, 294, Some(date));
        assert_eq!(url, truth);
    }
}
