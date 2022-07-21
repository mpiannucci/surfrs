use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Measurement {
    Length,
    Speed,
    Temperature,
    Pressure,
    Visibility,
    Direction,
    Time,
    WaveEnergy,
}

impl Measurement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Measurement::Length => "length",
            Measurement::Speed => "speed",
            Measurement::Temperature => "temperature",
            Measurement::Pressure => "pressure",
            Measurement::Visibility => "visibility",
            Measurement::Direction => "direction",
            Measurement::Time => "time",
            Measurement::WaveEnergy => "wave_energy"
        }
    }
}