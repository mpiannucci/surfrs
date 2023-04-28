use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gribberish::{message::Message, templates::product::tables::FixedSurfaceType};
use serde::{Deserialize, Serialize};

use crate::{
    dimensional_data::DimensionalData,
    location::Location,
    model::{GFSWaveModel, NOAAModel},
    swell::Swell,
    units::{Direction, Unit, UnitConvertible, UnitSystem},
};

use super::parseable_data_record::DataRecordParsingError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GFSWaveGribPointDataRecord {
    pub date: DateTime<Utc>,
    pub wave_summary: Swell,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub swell_components: Vec<Swell>,
}

impl GFSWaveGribPointDataRecord {
    pub fn from_messages(
        model: &GFSWaveModel,
        messages: &Vec<Message>,
        location: &Location,
        tolerance: f64,
    ) -> Result<Self, DataRecordParsingError> {
        let mut date: DateTime<Utc> = Utc::now();
        let mut data: HashMap<String, f64> = HashMap::new();

        messages.iter().for_each(|m| {
            let Ok(valid_time) = m.forecast_date() else {
                    return;
                };

            date = valid_time;

            let Ok(mut abbrev) = m.variable_abbrev() else {
                    return;
                };

            let Ok(level) = m.first_fixed_surface() else {
                    return;
                };

            match level.0 {
                FixedSurfaceType::OrderedSequence => {
                    abbrev = format!("{abbrev}_{}", level.1.map(|l| l as usize).unwrap_or(0))
                }
                _ => {}
            }

            match model.query_location_tolerance(location, &tolerance, m) {
                Ok(value) => {
                    let sum: f64 = value.iter().sum();
                    let mean: f64 = sum / value.len() as f64;
                    data.insert(abbrev, mean);
                }
                Err(err) => println!("{err}"),
            }
        });

        let Some(wave_height) = data.get("HTSGW") else {
            return Err(DataRecordParsingError::KeyMissing("HTSGW".into()));
        };

        let Some(period) = data.get("PERPW") else {
            return Err(DataRecordParsingError::KeyMissing("PERPW".into()));
        };

        let Some(wave_direction) = data.get("DIRPW") else {
            return Err(DataRecordParsingError::KeyMissing("DIRPW".into()));
        };

        let wave_summary = Swell::new(
            &UnitSystem::Metric,
            *wave_height,
            *period,
            Direction::from_degrees(*wave_direction as i32),
            None,
        );

        let wind_speed_value = data.get("WIND").map(|w| *w);
        let wind_speed = DimensionalData {
            value: wind_speed_value,
            variable_name: "wind speed".into(),
            unit: Unit::MetersPerSecond,
        };

        let wind_direction_value = data.get("WIND").map(|d| Direction::from_degrees(*d as i32));
        let wind_direction = DimensionalData {
            value: wind_direction_value,
            variable_name: "wind directions".into(),
            unit: Unit::Degrees,
        };

        let mut swell_components = vec![];
        for i in 0..=3 {
            let ht_key = format!("SWELL_{i}");
            let per_key = format!("SWPER_{i}");
            let dir_key = format!("SWDIR_{i}");
            if data.contains_key(&ht_key)
                && data.contains_key(&per_key)
                && data.contains_key(&dir_key)
                && !data[&ht_key].is_nan()
                && !data[&per_key].is_nan()
                && !data[&dir_key].is_nan()
            {
                let component = Swell::new(
                    &UnitSystem::Metric,
                    data[&ht_key],
                    data[&per_key],
                    Direction::from_degrees(data[&dir_key] as i32),
                    None,
                );

                swell_components.push(component);
            }
        }

        if data.contains_key("WVHGT")
            && data.contains_key("WVPER")
            && data.contains_key("WVDIR")
            && !data["WVHGT"].is_nan()
            && !data["WVPER"].is_nan()
            && !data["WVDIR"].is_nan()
        {
            let component = Swell::new(
                &UnitSystem::Metric,
                data["WVHGT"],
                data["WVPER"],
                Direction::from_degrees(data["WVDIR"] as i32),
                None,
            );

            swell_components.push(component);
        };

        swell_components.sort_by(|a, b| {
            b.wave_height
                .get_value()
                .partial_cmp(&a.wave_height.get_value())
                .unwrap()
        });

        Ok(GFSWaveGribPointDataRecord {
            date,
            wave_summary,
            wind_speed,
            wind_direction,
            swell_components,
        })
    }
}

impl UnitConvertible<GFSWaveGribPointDataRecord> for GFSWaveGribPointDataRecord {
    fn to_units(&mut self, new_units: &UnitSystem) {
        self.wind_speed.to_units(new_units);
        self.wave_summary.to_units(new_units);
        self.swell_components
            .iter_mut()
            .for_each(|c| c.to_units(new_units));
    }
}
