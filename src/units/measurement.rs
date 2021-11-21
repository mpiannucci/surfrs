
#[derive(Clone, Debug)]
pub enum Measurement {
    Length,
    Speed,
    Temperature,
    Pressure,
    Visibility,
    Direction,
    Time,
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
        }
    }
}