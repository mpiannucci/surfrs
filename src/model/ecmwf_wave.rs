use chrono::{prelude::*, Duration};
use gribberish::index::IndexEntry;
use serde::{Deserialize, Serialize};

use crate::{data::ecmwf_wave_field::ECMWFWaveField, tools::date::round_to_nearest_multiple_hour};

use super::{
    byte_range::{coalesce_ranges, ByteRange},
    GriddedModel,
};

/// Mirrors of ECMWF open data.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ECMWFDataSource {
    /// data.ecmwf.int, ECMWF's own portal
    ECMWF,
    /// Google Cloud Storage `ecmwf-open-data`
    GCP,
    /// AWS S3 `ecmwf-forecasts`
    AWS,
}

/// The wave products in ECMWF open data. Each is one kind of file.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ECMWFWaveProduct {
    /// IFS high resolution deterministic forecast (`ifs`, stream `wave`, type `fc`).
    IfsHres,
    /// IFS ensemble, perturbed members 1-50 in one file (`waef`, type `ef`).
    /// There is no published control.
    IfsEnsMembers,
    /// IFS ensemble probabilities (`waef`, type `ep`). 00z and 12z only.
    IfsEnsProbability,
    /// AIFS deterministic forecast (`aifs-single`, stream `wave`, type `fc`).
    AifsSingle,
    /// AIFS ensemble control (`aifs-ens`, stream `waef`, type `cf`).
    AifsEnsControl,
    /// AIFS ensemble perturbed members 1-50 in one file (`waef`, type `pf`).
    AifsEnsMembers,
    /// AIFS ensemble probabilities (`waef`, type `ep`).
    AifsEnsProbability,
}

impl ECMWFWaveProduct {
    fn model_directory(&self) -> &'static str {
        match self {
            ECMWFWaveProduct::IfsHres
            | ECMWFWaveProduct::IfsEnsMembers
            | ECMWFWaveProduct::IfsEnsProbability => "ifs",
            ECMWFWaveProduct::AifsSingle => "aifs-single",
            ECMWFWaveProduct::AifsEnsControl
            | ECMWFWaveProduct::AifsEnsMembers
            | ECMWFWaveProduct::AifsEnsProbability => "aifs-ens",
        }
    }

    fn stream(&self) -> &'static str {
        match self {
            ECMWFWaveProduct::IfsHres | ECMWFWaveProduct::AifsSingle => "wave",
            _ => "waef",
        }
    }

    fn file_type(&self) -> &'static str {
        match self {
            ECMWFWaveProduct::IfsHres | ECMWFWaveProduct::AifsSingle => "fc",
            ECMWFWaveProduct::IfsEnsMembers => "ef",
            ECMWFWaveProduct::AifsEnsControl => "cf",
            ECMWFWaveProduct::AifsEnsMembers => "pf",
            ECMWFWaveProduct::IfsEnsProbability | ECMWFWaveProduct::AifsEnsProbability => "ep",
        }
    }

    fn is_ifs(&self) -> bool {
        self.model_directory() == "ifs"
    }

    fn is_probability(&self) -> bool {
        self.file_type() == "ep"
    }

    /// Hours after the run starts by which every file is published, measured
    /// on the 2026-09-22 runs with some margin.
    fn publication_delay_hours(&self) -> i64 {
        match self {
            ECMWFWaveProduct::IfsHres => 8,
            ECMWFWaveProduct::IfsEnsMembers | ECMWFWaveProduct::IfsEnsProbability => 10,
            ECMWFWaveProduct::AifsSingle => 6,
            _ => 8,
        }
    }

    /// Hours between runs that publish this product.
    fn run_interval_hours(&self) -> u32 {
        match self {
            ECMWFWaveProduct::IfsEnsProbability => 12,
            _ => 6,
        }
    }
}

pub struct ECMWFWaveModel {
    pub product: ECMWFWaveProduct,
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

impl ECMWFWaveModel {
    pub fn ifs_hres() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::IfsHres,
            id: "ifs.wave.fc",
            name: "ECMWF IFS Wave",
            description: "ECMWF IFS high resolution wave forecast (ECWAM), 0.25 degree",
        }
    }

    pub fn ifs_ens_members() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::IfsEnsMembers,
            id: "ifs.waef.ef",
            name: "ECMWF IFS Wave Ensemble",
            description: "ECMWF IFS ensemble wave forecast, members 1-50, 0.25 degree",
        }
    }

    pub fn ifs_ens_probability() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::IfsEnsProbability,
            id: "ifs.waef.ep",
            name: "ECMWF IFS Wave Ensemble Probabilities",
            description: "ECMWF IFS ensemble wave height exceedance probabilities, 0.25 degree",
        }
    }

    pub fn aifs_single() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::AifsSingle,
            id: "aifs-single.wave.fc",
            name: "ECMWF AIFS Wave",
            description: "ECMWF AIFS deterministic wave forecast, 0.25 degree",
        }
    }

    pub fn aifs_ens_control() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::AifsEnsControl,
            id: "aifs-ens.waef.cf",
            name: "ECMWF AIFS Wave Ensemble Control",
            description: "ECMWF AIFS ensemble wave forecast, control member, 0.25 degree",
        }
    }

    pub fn aifs_ens_members() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::AifsEnsMembers,
            id: "aifs-ens.waef.pf",
            name: "ECMWF AIFS Wave Ensemble",
            description: "ECMWF AIFS ensemble wave forecast, members 1-50, 0.25 degree",
        }
    }

    pub fn aifs_ens_probability() -> Self {
        ECMWFWaveModel {
            product: ECMWFWaveProduct::AifsEnsProbability,
            id: "aifs-ens.waef.ep",
            name: "ECMWF AIFS Wave Ensemble Probabilities",
            description:
                "ECMWF AIFS ensemble wave height and period exceedance probabilities, 0.25 degree",
        }
    }
}

impl GriddedModel for ECMWFWaveModel {
    type Source = ECMWFDataSource;

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
        let adjusted = *date - Duration::hours(self.product.publication_delay_hours());
        round_to_nearest_multiple_hour(&adjusted, self.product.run_interval_hours())
    }

    fn forecast_hours(&self, run: &DateTime<Utc>) -> Vec<usize> {
        let long_run = run.hour() % 12 == 0;

        if self.product.is_probability() {
            // The 240h file holds steps 12-240 and the 360h file 252-360.
            return if self.product.is_ifs() && !long_run {
                vec![]
            } else {
                vec![240, 360]
            };
        }

        if self.product.is_ifs() {
            let mut hours: Vec<usize> = (0..=144).step_by(3).collect();
            if long_run {
                hours.extend((150..=360).step_by(6));
            }
            hours
        } else {
            (0..=360).step_by(6).collect()
        }
    }

    fn url_root(&self, source: &ECMWFDataSource) -> &'static str {
        match source {
            ECMWFDataSource::ECMWF => "https://data.ecmwf.int/forecasts",
            ECMWFDataSource::GCP => "https://storage.googleapis.com/ecmwf-open-data",
            ECMWFDataSource::AWS => "https://ecmwf-forecasts.s3.eu-central-1.amazonaws.com",
        }
    }

    fn create_url(
        &self,
        source: &ECMWFDataSource,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String {
        format!(
            "{}.grib2",
            self.create_path_stem(source, output_hour, query_date)
        )
    }

    fn create_index_url(
        &self,
        source: &ECMWFDataSource,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String {
        format!(
            "{}.index",
            self.create_path_stem(source, output_hour, query_date)
        )
    }
}

impl ECMWFWaveModel {
    fn create_path_stem(
        &self,
        source: &ECMWFDataSource,
        output_hour: usize,
        query_date: Option<DateTime<Utc>>,
    ) -> String {
        let base = self.url_root(source);
        let run = self.closest_model_run_date(&query_date.unwrap_or(Utc::now()));
        let date = run.format("%Y%m%d");
        let hour = run.hour();
        let model = self.product.model_directory();
        let stream = self.product.stream();
        let file_type = self.product.file_type();

        format!(
            "{base}/{date}/{hour:02}z/{model}/0p25/{stream}/{date}{hour:02}0000-{output_hour}h-{stream}-{file_type}"
        )
    }
}

/// Which ensemble members to select from an index. Entries without a member
/// number (deterministic, control and probability files) always match.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberSelection {
    #[default]
    All,
    Only(Vec<u8>),
}

/// Picks messages out of an ECMWF `.index` file (parsed with
/// `gribberish::index::parse_ecmwf_index`) so only those bytes are fetched.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ECMWFIndexQuery {
    /// Fields to keep. Empty keeps every field.
    pub fields: Vec<ECMWFWaveField>,
    pub members: MemberSelection,
    /// MARS steps to keep, e.g. `"24"` or `"120-168"`. None keeps every step.
    /// Probability files hold many steps each.
    pub steps: Option<Vec<String>>,
}

impl ECMWFIndexQuery {
    pub fn matches(&self, entry: &IndexEntry) -> bool {
        let param_matches = self.fields.is_empty()
            || entry.var.as_ref().map_or(false, |param| {
                self.fields.iter().any(|f| &f.mars_param() == param)
            });

        let member_matches = match (&self.members, entry.keys.get("number")) {
            (MemberSelection::All, _) | (_, None) => true,
            (MemberSelection::Only(members), Some(number)) => number
                .parse::<u8>()
                .map_or(false, |number| members.contains(&number)),
        };

        let step_matches = match (&self.steps, &entry.forecast_time) {
            (None, _) => true,
            (Some(steps), Some(step)) => steps.contains(step),
            (Some(_), None) => false,
        };

        param_matches && member_matches && step_matches
    }

    /// Byte ranges of the matching messages, in file order.
    pub fn select(&self, entries: &[IndexEntry]) -> Vec<ByteRange> {
        entries
            .iter()
            .filter(|e| self.matches(e))
            .filter_map(ByteRange::from_index_entry)
            .collect()
    }

    /// Like `select`, with adjacent messages merged into single requests.
    pub fn select_coalesced(&self, entries: &[IndexEntry]) -> Vec<ByteRange> {
        coalesce_ranges(self.select(entries), 0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn test_urls() {
        let run = at(2026, 9, 22, 8, 30);
        let model = ECMWFWaveModel::ifs_hres();
        assert_eq!(
            model.create_url(&ECMWFDataSource::GCP, 24, Some(run)),
            "https://storage.googleapis.com/ecmwf-open-data/20260922/00z/ifs/0p25/wave/20260922000000-24h-wave-fc.grib2"
        );
        assert_eq!(
            model.create_index_url(&ECMWFDataSource::ECMWF, 24, Some(run)),
            "https://data.ecmwf.int/forecasts/20260922/00z/ifs/0p25/wave/20260922000000-24h-wave-fc.index"
        );

        let model = ECMWFWaveModel::aifs_ens_members();
        assert_eq!(
            model.create_url(&ECMWFDataSource::AWS, 360, Some(run)),
            "https://ecmwf-forecasts.s3.eu-central-1.amazonaws.com/20260922/00z/aifs-ens/0p25/waef/20260922000000-360h-waef-pf.grib2"
        );
    }

    #[test]
    fn test_closest_model_run_date() {
        // IFS HRES 00z is complete by ~07:40Z.
        let model = ECMWFWaveModel::ifs_hres();
        assert_eq!(
            model.closest_model_run_date(&at(2026, 9, 22, 7, 59)),
            at(2026, 9, 21, 18, 0)
        );
        assert_eq!(
            model.closest_model_run_date(&at(2026, 9, 22, 8, 0)),
            at(2026, 9, 22, 0, 0)
        );

        // AIFS single is published sooner.
        let model = ECMWFWaveModel::aifs_single();
        assert_eq!(
            model.closest_model_run_date(&at(2026, 9, 22, 6, 0)),
            at(2026, 9, 22, 0, 0)
        );

        // IFS ensemble probabilities only come from the 00z and 12z runs.
        let model = ECMWFWaveModel::ifs_ens_probability();
        assert_eq!(
            model.closest_model_run_date(&at(2026, 9, 22, 20, 0)),
            at(2026, 9, 22, 0, 0)
        );
        assert_eq!(
            model.closest_model_run_date(&at(2026, 9, 22, 22, 0)),
            at(2026, 9, 22, 12, 0)
        );
    }

    #[test]
    fn test_forecast_hours() {
        let long_run = at(2026, 9, 22, 0, 0);
        let short_run = at(2026, 9, 22, 6, 0);

        let hours = ECMWFWaveModel::ifs_hres().forecast_hours(&long_run);
        assert_eq!(hours.len(), 85);
        assert_eq!(&hours[47..51], &[141, 144, 150, 156]);
        assert_eq!(hours.last(), Some(&360));

        let hours = ECMWFWaveModel::ifs_ens_members().forecast_hours(&short_run);
        assert_eq!(hours.len(), 49);
        assert_eq!(hours.last(), Some(&144));

        let hours = ECMWFWaveModel::aifs_single().forecast_hours(&short_run);
        assert_eq!(hours.len(), 61);
        assert_eq!(hours[1], 6);

        assert_eq!(
            ECMWFWaveModel::ifs_ens_probability().forecast_hours(&long_run),
            vec![240, 360]
        );
        assert!(ECMWFWaveModel::ifs_ens_probability()
            .forecast_hours(&short_run)
            .is_empty());
        assert_eq!(
            ECMWFWaveModel::aifs_ens_probability().forecast_hours(&short_run),
            vec![240, 360]
        );
    }
}
