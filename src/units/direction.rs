use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use super::CardinalDirection;
use super::Unit;
use super::DataParseError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectionConvention {
    From, 
    Towards,
    Met,
}

impl DirectionConvention {
    /// Normalizes direction to From convention in degrees
    pub fn normalize(&self, dir: f64) -> f64 {
        match self {
            DirectionConvention::From => dir,
            DirectionConvention::Towards => (dir + 180.0) % 360.0,
            DirectionConvention::Met => ((270.0 - dir) + 360.0) % 360.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Direction {
    pub degrees: i32,
    direction: CardinalDirection,
}

impl Direction {
    pub fn from_cardinal_direction(direction: CardinalDirection) -> Direction {
        Direction {
            direction: direction.clone(),
            degrees: direction.to_degrees(),
        }
    }

    pub fn from_degrees(degree: i32) -> Direction {
        Direction {
            direction: CardinalDirection::from_degrees(&degree),
            degrees: degree.clone(),
        }
    }

    pub fn from_radians(radians: f64) -> Direction {
        let degrees = radians.to_degrees() as i32;
        Direction { 
            direction: CardinalDirection::from_degrees(&degrees), 
            degrees
        }
    }

    pub fn flip(&mut self) {
        let degrees = (self.degrees + 180) % 360;
        self.degrees = degrees;
        self.direction = CardinalDirection::from_degrees(&degrees);
    }

    pub fn cardinal_direction(&self) -> &CardinalDirection {
        &self.direction
    }

    pub fn radian(&self) -> f64 {
        (self.degrees as f64).to_radians()
    }

    pub fn invert(&self) -> Direction {
        Direction::from_degrees((self.degrees + 180) % 360)
    }

    pub fn is_opposite(&self, other: &Direction) -> bool {
        let diff = (self.degrees - other.degrees).abs();
        diff >= 170 && diff <= 190
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            self.degrees,
            Unit::Degrees.abbreviation(),
            self.direction
        )
    }
}

impl FromStr for Direction {
    type Err = DataParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_cardinal: Result<CardinalDirection, DataParseError> = s.parse();
        match parse_cardinal {
            Ok(dir) => Ok(Direction {
                direction: dir.clone(),
                degrees: dir.to_degrees(),
            }),
            Err(_) => {
                let parse_direction = s.parse::<i32>();
                match parse_direction {
                    Ok(dir) => Ok(Direction::from_degrees(dir)),
                    Err(_) => Err(DataParseError::InvalidString),
                }
            }
        }
    }
}