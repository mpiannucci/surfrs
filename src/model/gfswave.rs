use chrono::prelude::*;

use crate::tools::date::closest_gfs_model_gridded_datetime;

use super::{GriddedModel, ModelTimeOutputResolution, NOAADataSource};

pub struct GFSWaveModel {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
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

    pub fn time_resolution(&self) -> ModelTimeOutputResolution {
        ModelTimeOutputResolution::HybridHourlyThreeHourly(120)
    }
}

impl GriddedModel for GFSWaveModel {
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
            NOAADataSource::NODDAWS => "https://noaa-gfs-bdp-pds.s3.amazonaws.com",
            NOAADataSource::NOMADS => "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod",
            NOAADataSource::NODDGCP => "https://storage.googleapis.com/global-forecast-system",
        }
    }

    fn create_url(
        &self,
        source: &NOAADataSource,
        output_hour: usize,
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
            "{base}/gfs.{year}{month:02}{day:02}/{hour:02}/wave/gridded/gfswave.t{hour:02}z.{id}.f{output_hour:03}.grib2"
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use crate::model::GFSWaveModel;

    use super::{GriddedModel, NOAADataSource};

    #[test]
    fn test_gfs_wave_url() {
        let truth = "https://storage.googleapis.com/global-forecast-system/gfs.20230117/06/wave/gridded/gfswave.t06z.atlocn.0p16.f115.grib2";

        let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 01, 17, 13, 0, 0).unwrap();

        let gfs_wave = GFSWaveModel::atlantic();
        let url = gfs_wave.create_url(&NOAADataSource::NODDGCP, 115, Some(date));
        assert_eq!(url, truth);

        let truth = "https://storage.googleapis.com/global-forecast-system/gfs.20230117/06/wave/gridded/gfswave.t06z.atlocn.0p16.f126.grib2";
        let url = gfs_wave.create_url(&NOAADataSource::NODDGCP, 126, Some(date));
        assert_eq!(url, truth);
    }
}
