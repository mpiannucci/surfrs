use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use csv::Reader;
use geojson::{Feature, FeatureCollection};
use serde::{Deserialize, Serialize};

use crate::{
    buoy_station::BuoyStation,
    dimensional_data::DimensionalData,
    swell::{Swell, SwellProvider, SwellSummary},
    units::{Direction, Unit, UnitConvertible, UnitSystem},
};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatestObsDataRecord {
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub date: DateTime<Utc>,
    pub wind_direction: DimensionalData<Direction>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_gust_speed: DimensionalData<f64>,
    pub wave_height: DimensionalData<f64>,
    pub dominant_wave_period: DimensionalData<f64>,
    pub average_wave_period: DimensionalData<f64>,
    pub mean_wave_direction: DimensionalData<Direction>,
    pub air_pressure: DimensionalData<f64>,
    pub air_pressure_tendency: DimensionalData<f64>,
    pub air_temperature: DimensionalData<f64>,
    pub water_temperature: DimensionalData<f64>,
    pub dewpoint_temperature: DimensionalData<f64>,
    pub visibility: DimensionalData<f64>,
    pub tide: DimensionalData<f64>,
}

impl LatestObsDataRecord {
    pub fn station(&self) -> BuoyStation {
        BuoyStation::new(self.station_id.clone(), self.latitude, self.longitude)
    }
}

// #STN     LAT      LON  YYYY MM DD hh mm WDIR WSPD   GST WVHT  DPD APD MWD   PRES  PTDY  ATMP  WTMP  DEWP  VIS   TIDE
impl ParseableDataRecord for LatestObsDataRecord {
    type Metadata = ();

    fn from_data_row(
        _: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<LatestObsDataRecord, DataRecordParsingError> {
        let station_id = row[0].to_string();
        let latitude = row[1].parse().unwrap();
        let longitude = row[2].parse().unwrap();
        let date = Utc
            .with_ymd_and_hms(
                row[3].parse().unwrap(),
                row[4].parse().unwrap(),
                row[5].parse().unwrap(),
                row[6].parse().unwrap(),
                row[7].parse().unwrap(),
                0,
            )
            .unwrap();

        Ok(LatestObsDataRecord {
            station_id,
            latitude,
            longitude,
            date,
            wind_direction: DimensionalData::from_raw_data(
                row[8],
                "wind direction".into(),
                Unit::Degrees,
            ),
            wind_speed: DimensionalData::from_raw_data(
                row[9],
                "wind speed".into(),
                Unit::MetersPerSecond,
            ),
            wind_gust_speed: DimensionalData::from_raw_data(
                row[10],
                "wind gust speed".into(),
                Unit::MetersPerSecond,
            ),
            wave_height: DimensionalData::from_raw_data(
                row[11],
                "wave height".into(),
                Unit::Meters,
            ),
            dominant_wave_period: DimensionalData::from_raw_data(
                row[12],
                "dominant wave period".into(),
                Unit::Seconds,
            ),
            average_wave_period: DimensionalData::from_raw_data(
                row[13],
                "average wave period".into(),
                Unit::Seconds,
            ),
            mean_wave_direction: DimensionalData::from_raw_data(
                row[14],
                "mean wave direction".into(),
                Unit::Degrees,
            ),
            air_pressure: DimensionalData::from_raw_data(
                row[15],
                "air pressure".into(),
                Unit::HectaPascal,
            ),
            air_pressure_tendency: DimensionalData::from_raw_data(
                row[16],
                "air pressure tendency".into(),
                Unit::HectaPascal,
            ),
            air_temperature: DimensionalData::from_raw_data(
                row[17],
                "air temperature".into(),
                Unit::Celsius,
            ),
            water_temperature: DimensionalData::from_raw_data(
                row[18],
                "water temperature".into(),
                Unit::Celsius,
            ),
            dewpoint_temperature: DimensionalData::from_raw_data(
                row[19],
                "dewpoint temperature".into(),
                Unit::Celsius,
            ),
            visibility: DimensionalData::from_raw_data(row[20], "".into(), Unit::NauticalMiles),
            tide: DimensionalData::from_raw_data(row[21], "tide".into(), Unit::Feet),
        })
    }
}

impl UnitConvertible for LatestObsDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.wind_direction.to_units(new_units);
        self.wind_speed.to_units(new_units);
        self.wind_gust_speed.to_units(new_units);
        self.wave_height.to_units(new_units);
        self.dominant_wave_period.to_units(new_units);
        self.average_wave_period.to_units(new_units);
        self.mean_wave_direction.to_units(new_units);
        self.air_pressure.to_units(new_units);
        self.air_pressure_tendency.to_units(new_units);
        self.air_temperature.to_units(new_units);
        self.water_temperature.to_units(new_units);
        self.dewpoint_temperature.to_units(new_units);
        self.visibility.to_units(new_units);
        self.tide.to_units(new_units);

        self
    }
}

impl SwellProvider for LatestObsDataRecord {
    fn swell_data(&self) -> Result<SwellSummary, crate::swell::SwellProviderError> {
        Ok(SwellSummary {
            summary: Swell {
                wave_height: self.wave_height.clone(),
                period: self.dominant_wave_period.clone(),
                direction: self.mean_wave_direction.clone(),
                energy: None,
                spectral_density: None,
                partition: None,
            },
            components: vec![],
        })
    }
}

pub struct LatestObsDataRecordCollection<'a> {
    reader: Reader<&'a [u8]>,
}

impl<'a> LatestObsDataRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        let reader = csv::ReaderBuilder::new()
            .delimiter(b' ')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(data.as_bytes());

        LatestObsDataRecordCollection { reader }
    }

    pub fn records(&'a mut self) -> impl Iterator<Item = LatestObsDataRecord> + 'a {
        self.reader
            .records()
            .map(
                |result| -> Result<LatestObsDataRecord, DataRecordParsingError> {
                    match result {
                        Ok(record) => {
                            let filtered_record: Vec<&str> =
                                record.iter().filter(|data| !data.is_empty()).collect();
                            let mut met_data =
                                LatestObsDataRecord::from_data_row(None, &filtered_record)?;
                            met_data.to_units(&UnitSystem::Metric);
                            Ok(met_data)
                        }
                        Err(e) => Err(DataRecordParsingError::ParseFailure(e.to_string())),
                    }
                },
            )
            .filter_map(|d| d.ok())
    }
}

pub fn latest_obs_feature_collection<'a>(
    buoy_stations: &'a Vec<BuoyStation>,
    latest_obs: &'a Vec<LatestObsDataRecord>,
) -> FeatureCollection {
    let latest_obs_map = latest_obs
        .iter()
        .map(|lo| (lo.station_id.clone(), lo.clone()))
        .collect::<HashMap<String, LatestObsDataRecord>>();

    let features = buoy_stations
        .iter()
        .map(|b| {
            let mut station_feature: Feature = b.clone().into();

            if let Some(latest_obs) = latest_obs_map.get(&b.station_id) {
                let observation_data_value = serde_json::to_value(&latest_obs).unwrap();
                let mut observation_data = observation_data_value.as_object().unwrap().clone();
                observation_data.remove("station_id");
                observation_data.remove("latitude");
                observation_data.remove("longitude");

                observation_data.retain(|_, v| {
                    if let Some(v_obj) = v.as_object() {
                        match v_obj.get("value") {
                            Some(vv) => !vv.is_null(),
                            None => true,
                        }
                    } else {
                        true
                    }
                });

                station_feature.set_property("latest_observations", observation_data);
            }

            station_feature
        })
        .collect();

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_latest_obs_row_parse() {
        let raw_data = "44097  40.967  -71.124 2022 12 30 01 26  MM    MM    MM  1.7   6  4.9 212     MM    MM    MM  10.3    MM   MM     MM";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let met_data = LatestObsDataRecord::from_data_row(None, &data_row).unwrap();

        assert_eq!(met_data.date.year(), 2022);
        assert_eq!(met_data.station_id, "44097");
        assert_eq!(met_data.wave_height.value.unwrap(), 1.7);
        assert_eq!(met_data.mean_wave_direction.value.unwrap().degrees, 212);
        assert_eq!(met_data.water_temperature.value.unwrap(), 10.3);
        assert!(met_data.tide.value.is_none());
    }
}
