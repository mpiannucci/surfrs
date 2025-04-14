pub mod cardinal_direction;
pub mod direction;
pub mod steepness;

pub use cardinal_direction::CardinalDirection;
pub use direction::Direction;
use serde::{Deserialize, Serialize};
pub use steepness::Steepness;

use std::fmt::{self, Display};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    Millimeters,
    Meters,
    MetersPerSecond,
    Celsius,
    Pascal,
    HectaPascal,
    Inches,
    Feet,
    MilesPerHour,
    Fahrenheit,
    InchesMercury,
    Knots,
    Kelvin,
    MetersSquaredPerHertz,
    NauticalMiles,
    Degrees,
    Seconds,
    Percent,
    KiloJoules,
    Unknown,
}

impl Unit {
    pub fn abbreviation(&self) -> &'static str {
        match self {
            Unit::Millimeters => "mm",
            Unit::Meters => "m",
            Unit::MetersPerSecond => "m/s",
            Unit::Celsius => "°C",
            Unit::Pascal => "pa",
            Unit::HectaPascal => "hpa",
            Unit::Inches => "in",
            Unit::Feet => "ft",
            Unit::MilesPerHour => "mph",
            Unit::Fahrenheit => "°F",
            Unit::InchesMercury => "in Hg",
            Unit::Knots => "kt",
            Unit::Kelvin => "K",
            Unit::MetersSquaredPerHertz => "m²/Hz",
            Unit::NauticalMiles => "nmi",
            Unit::Degrees => "°",
            Unit::Seconds => "s",
            Unit::Percent => "%",
            Unit::KiloJoules => "kJ",
            Unit::Unknown => "",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Unit::Millimeters => "millimeters",
            Unit::Meters => "meters",
            Unit::MetersPerSecond => "meters per second",
            Unit::Celsius => "degrees celsius",
            Unit::Pascal => "pascal",
            Unit::HectaPascal => "hecta pascal",
            Unit::Inches => "inches",
            Unit::Feet => "feet",
            Unit::MilesPerHour => "miles per hour",
            Unit::Fahrenheit => "degrees fahrenheit",
            Unit::InchesMercury => "inches mercury",
            Unit::Knots => "knots",
            Unit::Kelvin => "kelvin",
            Unit::MetersSquaredPerHertz => "meters squared per hertz",
            Unit::NauticalMiles => "nautical miles",
            Unit::Degrees => "degrees",
            Unit::Seconds => "seconds",
            Unit::Percent => "percent",
            Unit::KiloJoules => "kilojoules",
            Unit::Unknown => "unknown",
        }
    }
}

impl From<&str> for Unit {
    fn from(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "mm" | "millimeters" | "millimeter" => Unit::Millimeters,
            "m" | "meters" | "meter" | "wmounit:m" => Unit::Meters,
            "m/s" | "mps" | "ms-1" | "meterspersecond" | "meterpersecond" => Unit::MetersPerSecond,
            "°c" | "degcelsius" | "degreecelsius" | "degreescelsius" | "wmounit:degc" => {
                Unit::Celsius
            }
            "pa" | "pascals" | "pascal" => Unit::Pascal,
            "hpa" | "hectapascals" | "hectapascal" => Unit::HectaPascal,
            "in" | "inches" | "inch" => Unit::Inches,
            "ft" | "feet" | "foot" => Unit::Feet,
            "mph" | "m/h" | "mh-1" | "milesperhour" => Unit::MilesPerHour,
            "°f" | "f" | "degfahrenheit" | "degreesfahrenheit" | "degreefahrenheit" => {
                Unit::Fahrenheit
            }
            "inhg" | "inches mercury" => Unit::InchesMercury,
            "kt" | "kts" | "knots" | "knot" => Unit::Knots,
            "k" | "kelvin" => Unit::Kelvin,
            "m^2/hz" | "m2hz-1" | "meterssquaredperhertz" => Unit::MetersSquaredPerHertz,
            "nmi" | "nauticalmiles" | "nauticalmile" => Unit::NauticalMiles,
            "°" | "deg" | "degs" | "degrees" | "degree" => Unit::Degrees,
            "s" | "second" | "seconds" => Unit::Seconds,
            "%" | "percent" | "percentage" | "wmounit:percent" => Unit::Percent,
            "kj" | "kilojoules" | "kilojoule" => Unit::KiloJoules,
            _ => Unit::Unknown,
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.abbreviation())
    }
}

impl Unit {
    pub fn convert(&self, value: f64, target: &Unit) -> f64 {
        match self {
            Unit::Millimeters => match target {
                Unit::Meters => value * 0.001,
                Unit::Inches => value / 25.4,
                _ => value,
            },
            Unit::Meters => match target {
                Unit::Millimeters => value * 1000.0,
                Unit::Feet => value * 3.281,
                _ => value,
            },
            Unit::MetersPerSecond => match target {
                Unit::MilesPerHour => value * 2.237,
                Unit::Knots => value * 1.944,
                _ => value,
            },
            Unit::Celsius => match target {
                Unit::Fahrenheit => value * (9.0 / 5.0) + 32.0,
                Unit::Kelvin => value + 273.0,
                _ => value,
            },
            Unit::Pascal => match target {
                Unit::HectaPascal => value / 100.0,
                _ => value,
            },
            Unit::HectaPascal => match target {
                Unit::Pascal => value * 100.0,
                Unit::InchesMercury => value / 33.8638,
                _ => value,
            },
            Unit::Inches => match target {
                Unit::Feet => value / 12.0,
                Unit::Millimeters => value * 25.4,
                _ => value,
            },
            Unit::Feet => match target {
                Unit::Inches => value * 12.0,
                Unit::Meters => value / 3.281,
                _ => value,
            },
            Unit::MilesPerHour => match target {
                Unit::MetersPerSecond => value / 2.237,
                Unit::Knots => value / 1.15,
                _ => value,
            },
            Unit::Fahrenheit => match target {
                Unit::Celsius => value - 32.0 * (5.0 / 9.0),
                Unit::Kelvin => (value + 459.67) * (5.0 / 9.0),
                _ => value,
            },
            Unit::InchesMercury => match target {
                Unit::HectaPascal => value * 33.8638,
                _ => value,
            },
            Unit::Knots => match target {
                Unit::MetersPerSecond => value * 0.514,
                Unit::MilesPerHour => value * 1.15,
                _ => value,
            },
            Unit::Kelvin => match target {
                Unit::Celsius => value - 273.0,
                Unit::Fahrenheit => value * (9.0 / 5.0) - 459.67,
                _ => value,
            },
            _ => value,
        }
    }

    pub fn convert_system(&self, target_system: &UnitSystem) -> Unit {
        match self {
            Unit::Millimeters => match target_system {
                UnitSystem::English => Unit::Inches,
                _ => self.clone(),
            },
            Unit::Meters => match target_system {
                UnitSystem::English => Unit::Feet,
                _ => self.clone(),
            },
            Unit::MetersPerSecond => match target_system {
                UnitSystem::English => Unit::MilesPerHour,
                UnitSystem::Knots => Unit::Knots,
                _ => self.clone(),
            },
            Unit::Celsius => match target_system {
                UnitSystem::English => Unit::Fahrenheit,
                UnitSystem::Kelvin => Unit::Kelvin,
                _ => self.clone(),
            },
            Unit::HectaPascal => match target_system {
                UnitSystem::English => Unit::InchesMercury,
                _ => self.clone(),
            },
            Unit::Inches => match target_system {
                UnitSystem::Metric => Unit::Millimeters,
                _ => self.clone(),
            },
            Unit::Feet => match target_system {
                UnitSystem::Metric => Unit::Meters,
                _ => self.clone(),
            },
            Unit::MilesPerHour => match target_system {
                UnitSystem::Metric => Unit::MetersPerSecond,
                UnitSystem::Knots => Unit::Knots,
                _ => self.clone(),
            },
            Unit::Fahrenheit => match target_system {
                UnitSystem::Metric => Unit::Celsius,
                UnitSystem::Kelvin => Unit::Kelvin,
                _ => self.clone(),
            },
            Unit::InchesMercury => match target_system {
                UnitSystem::Metric => Unit::HectaPascal,
                _ => self.clone(),
            },
            Unit::Knots => match target_system {
                UnitSystem::Metric => Unit::MetersPerSecond,
                UnitSystem::English => Unit::MilesPerHour,
                _ => self.clone(),
            },
            Unit::Kelvin => match target_system {
                UnitSystem::Metric => Unit::Celsius,
                UnitSystem::English => Unit::Fahrenheit,
                _ => self.clone(),
            },
            _ => self.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnitSystem {
    Metric,
    English,
    Knots,
    Kelvin,
}

impl UnitSystem {
    pub fn earths_radius(&self) -> f64 {
        match self {
            UnitSystem::Metric => 6371.0,
            UnitSystem::English => 3956.0,
            _ => 0.0,
        }
    }

    pub fn density_of_seawater(&self) -> f64 {
        match self {
            UnitSystem::Metric => 1029.0,
            UnitSystem::English => 64.0,
            _ => 0.0,
        }
    }
}

impl Display for UnitSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let as_str = match self {
            UnitSystem::Metric => "metric",
            UnitSystem::English => "english",
            UnitSystem::Knots => "knots",
            UnitSystem::Kelvin => "kelvin",
        };

        write!(f, "{as_str}")
    }
}

pub enum DataParseError {
    InvalidString,
}

pub trait UnitConvertible {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self;
}
