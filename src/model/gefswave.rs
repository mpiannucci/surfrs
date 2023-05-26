use chrono::prelude::*;

use crate::tools::date::{closest_gfs_model_gridded_datetime};

use super::{ModelDataSource, ModelTimeOutputResolution, NOAAModel};

pub struct GEFSWaveModel {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

impl GEFSWaveModel {
    pub fn global_16_mean() -> Self {
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
}

impl NOAAModel for GEFSWaveModel {
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

    fn time_resolution(&self) -> ModelTimeOutputResolution {
        ModelTimeOutputResolution::HybridThreeHourlySixHourly(240)
    }

    fn url_root(&self, source: &ModelDataSource) -> &'static str {
        match source {
            ModelDataSource::NODDAWS => "https://noaa-gefs-pds.s3.amazonaws.com",
            ModelDataSource::NOMADS => "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gefs/prod",
            ModelDataSource::NODDGCP => "",
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

    use super::{ModelDataSource, NOAAModel, GEFSWaveModel};

    #[test]
    fn test_gefs_wave_url() {
        let truth = "https://noaa-gefs-pds.s3.amazonaws.com/gefs.20230525/06/wave/gridded/gefs.wave.t06z.spread.global.0p25.f216.grib2";

        let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 05, 25, 13, 0, 0).unwrap();

        let gefs_wave = GEFSWaveModel::global_25_spread();
        let url = gefs_wave.create_url(&ModelDataSource::NODDAWS, 216, Some(date));
        assert_eq!(url, truth);

        let truth = "https://noaa-gefs-pds.s3.amazonaws.com/gefs.20230525/06/wave/gridded/gefs.wave.t06z.spread.global.0p25.f294.grib2";
        let url = gefs_wave.create_url(&ModelDataSource::NODDAWS, 294, Some(date));
        assert_eq!(url, truth);
    }
}
