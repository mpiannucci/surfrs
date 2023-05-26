use chrono::prelude::*;

use crate::tools::date::{closest_gfs_model_gridded_datetime};

use super::{NOAAModel, ModelTimeOutputResolution};

pub struct NWPSModel {
    id: &'static str,
    region: &'static str,
    name: &'static str,
    description: &'static str,
}

impl NWPSModel {
    pub fn boston() -> Self {
        NWPSModel {
            id: "box",
            region: "er",
            name: "NWPS Boston",
            description: "NOAA Nearshore Wave Predication System: Boston",
        }
    }
}

impl NOAAModel for NWPSModel {
    fn id(&self) -> &'static str {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn time_resolution(&self) -> ModelTimeOutputResolution {
        ModelTimeOutputResolution::Hourly
    }

    fn closest_model_run_date(&self, date: &chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
        closest_gfs_model_gridded_datetime(date)
    }

    fn url_root(&self, _: &super::ModelDataSource) -> &'static str {
        // At this time, only NOMADS is supported, hopefully NODD will be supported in the future.
        "https://nomads.ncep.noaa.gov/pub/data/nccf/com/nwps/prod/"
    }

    fn create_url(
        &self,
        source: &super::ModelDataSource,
        _: usize,
        model_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> String {
        let base = self.url_root(source);
        let id = self.id;
        let region = self.region;
        let model_date = self.closest_model_run_date(&model_date.unwrap_or(Utc::now()));
        let year = model_date.year();
        let month = model_date.month();
        let day = model_date.day();
        let hour = model_date.hour();

        format!(
            "{base}{region}.{year}{month:02}{day:02}/{id}/{hour:02}/CG1/{id}_nwps_CG1_{year}{month:02}{day:02}_{hour:02}00.grib2"
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use crate::model::NWPSModel;

    use super::{NOAAModel};
    use crate::model::ModelDataSource;

    #[test]
    fn test_gfs_wave_url() {
        let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 03, 11, 13, 0, 0).unwrap();
        let box_nwps = NWPSModel::boston();

        let truth = "https://nomads.ncep.noaa.gov/pub/data/nccf/com/nwps/prod/er.20230311/box/06/CG1/box_nwps_CG1_20230311_0600.grib2";
        let url = box_nwps.create_url(&ModelDataSource::NOMADS, 0, Some(date));
        assert_eq!(url, truth);
    }
}
