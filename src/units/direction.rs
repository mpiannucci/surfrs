use std::fmt;
use std::str::FromStr;

use super::Measurement;
use super::CardinalDirection;
use super::Units;
use super::DataParseError;

#[derive(Clone, Debug)]
pub struct Direction {
    pub direction: CardinalDirection,
    pub degree: Option<i64>,
}

impl Direction {
    pub fn from_degree(degree: i64) -> Direction {
        Direction {
            direction: CardinalDirection::from_degree(degree),
            degree: Some(degree),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.degree {
            Some(degree) => write!(
                f,
                "{}{} {}",
                degree,
                Units::Metric.label(&Measurement::Direction, true),
                self.direction
            ),
            None => write!(f, "{}", self.direction),
        }
    }
}

impl FromStr for Direction {
    type Err = DataParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_cardinal: Result<CardinalDirection, DataParseError> = s.parse();
        match parse_cardinal {
            Ok(dir) => Ok(Direction {
                direction: dir,
                degree: None,
            }),
            Err(_) => {
                let parse_direction = s.parse::<i64>();
                match parse_direction {
                    Ok(dir) => Ok(Direction::from_degree(dir)),
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