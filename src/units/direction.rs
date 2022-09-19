use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use super::Measurement;
use super::CardinalDirection;
use super::Units;
use super::DataParseError;

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
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            self.degrees,
            Units::Metric.label(&Measurement::Direction, true),
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
                direction: dir,
                // TODO
                degrees: 0,
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

impl From<Direction> for Measurement {
    fn from(_: Direction) -> Measurement {
        Measurement::Direction
    }
}