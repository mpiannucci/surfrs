use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gribberish::{
    message::{Message},
    templates::product::tables::FixedSurfaceType,
};
use serde::{Deserialize, Serialize};

use crate::{
    dimensional_data::DimensionalData,
    location::Location,
    model::{GFSWaveModel, NOAAModel},
    swell::Swell,
    units::{Direction, Units, Measurement},
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

            match model.query_location_data(location, m) {
                Ok(value) => {data.insert(abbrev, value);},
                Err(err) => println!("{err}"),
            }
        });

        let wave_summary = Swell::new(
            &Units::Metric, 
            data["HTSGW"], 
            data["PERPW"], 
            Direction::from_degrees(data["DIRPW"] as i32), 
            None
        );

        let wind_speed = DimensionalData {
            value: Some(data["WIND"]),
            variable_name: "wind speed".into(),
            measurement: Measurement::Speed,
            unit: Units::Metric,
        };

        let wind_direction = DimensionalData {
            value: Some(Direction::from_degrees(data["WDIR"] as i32)),
            variable_name: "wind directions".into(),
            measurement: Measurement::Direction,
            unit: Units::Metric,
        };

        let mut swell_components = vec![];
        for i in 0..=3 {
            let ht_key = format!("SWELL_{i}");
            let per_key = format!("SWPER_{i}");
            let dir_key = format!("SWDIR_{i}");
            if data.contains_key(&ht_key) && data.contains_key(&per_key) && data.contains_key(&dir_key) {
                let component = Swell::new(
                    &Units::Metric,
                    data[&ht_key],
                    data[&per_key], 
                    Direction::from_degrees(data[&dir_key] as i32),
                    None,
                );

                swell_components.push(component);
            }
        }

        if data.contains_key("WVHGT") && data.contains_key("WVPER") && data.contains_key("WVDIR") {
            let component = Swell::new(
                &Units::Metric,
                data["WVHGT"],
                data["WVPER"], 
                Direction::from_degrees(data["WVDIR"] as i32),
                None,
            );

            swell_components.push(component);
        };
        
        GFSWaveGribPointDataRecord {
            date,
            wave_summary, 
            wind_speed, 
            wind_direction, 
            swell_components,
        }
    }
}
