use crate::data::dimensional_data::DimensionalData;
use crate::data::units::*;

use super::date_record::DateRecord;
use super::parseable_data_record::ParseableDataRecord;

#[derive(Clone, Debug)]
pub struct MeteorologicalDataRecord {
    pub date: DateRecord,
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

impl ParseableDataRecord for MeteorologicalDataRecord {
    fn from_data_row(row: &Vec<&str>) -> MeteorologicalDataRecord {
        MeteorologicalDataRecord {
            date: DateRecord::from_data_row(row),
            wind_direction: DimensionalData::from_raw_data(row[5], "wind direction", Measurement::Direction, Units::Metric),
            wind_speed: DimensionalData::from_raw_data(row[6], "wind speed", Measurement::Speed, Units::Metric),
            wind_gust_speed: DimensionalData::from_raw_data(row[7], "wind gust speed", Measurement::Speed, Units::Metric),
            wave_height: DimensionalData::from_raw_data(row[8], "wave height", Measurement::Length, Units::Metric),
            dominant_wave_period: DimensionalData::from_raw_data(row[9], "dominant wave period", Measurement::Time, Units::Metric),
            average_wave_period: DimensionalData::from_raw_data(row[10], "average wave period", Measurement::Time, Units::Metric),
            mean_wave_direction: DimensionalData::from_raw_data(row[11], "mean wave direction", Measurement::Direction, Units::Metric),
            air_pressure: DimensionalData::from_raw_data(row[12], "air pressure", Measurement::Pressure, Units::Metric),  
            air_temperature: DimensionalData::from_raw_data(row[13], "air temperature", Measurement::Temperature, Units::Metric),
            water_temperature: DimensionalData::from_raw_data(row[14], "water temperature", Measurement::Temperature, Units::Metric),
            dewpoint_temperature: DimensionalData::from_raw_data(row[15], "dewpoint temperature", Measurement::Temperature, Units::Metric),
            visibility: DimensionalData::from_raw_data(row[16], "", Measurement::Visibility, Units::Metric),
            air_pressure_tendency: DimensionalData::from_raw_data(row[17], "air pressure tendency", Measurement::Pressure, Units::Metric),
            tide: DimensionalData::from_raw_data(row[18], "tide", Measurement::Length, Units::English),
        }
    }
}

impl UnitConvertible<MeteorologicalDataRecord> for MeteorologicalDataRecord {
    fn to_units(&mut self, new_units: &Units) {
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
    }
}