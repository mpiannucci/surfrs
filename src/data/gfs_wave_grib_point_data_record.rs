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
    ) -> Self {
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

            match model.interp_location_data(location, m) {
                Ok(value) => {
                    data.insert(abbrev, value);
                }
                Err(err) => println!("{err}"),
            }
        });

        let wave_summary = Swell::new(
            &UnitSystem::Metric,
            data["HTSGW"],
            data["PERPW"],
            Direction::from_degrees(data["DIRPW"] as i32),
            None,
        );

        let wind_speed = DimensionalData {
            value: Some(data["WIND"]),
            variable_name: "wind speed".into(),
            unit: Unit::MetersPerSecond,
        };

        let wind_direction = DimensionalData {
            value: Some(Direction::from_degrees(data["WDIR"] as i32)),
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

        GFSWaveGribPointDataRecord {
            date,
            wave_summary,
            wind_speed,
            wind_direction,
            swell_components,
        }
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
