use std::fmt;
use std::str::FromStr;

use super::DataParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum CardinalDirection {
    North,
    NorthNorthEast,
    NorthEast,
    EastNorthEast,
    East,
    EastSouthEast,
    SouthEast,
    SouthSouthEast,
    South,
    SouthSouthWest,
    SouthWest,
    WestSouthWest,
    West,
    WestNorthWest,
    NorthWest,
    NorthNorthWest,
    Invalid,
}

impl CardinalDirection {
    pub fn from_degree(degree: i64) -> CardinalDirection {
        match degree {
            349..=360 | 0..=11 => CardinalDirection::North,
            12..=33 => CardinalDirection::NorthNorthEast,
            34..=56 => CardinalDirection::NorthEast,
            57..=78 => CardinalDirection::EastNorthEast,
            79..=101 => CardinalDirection::East,
            102..=123 => CardinalDirection::EastSouthEast,
            124..=146 => CardinalDirection::SouthEast,
            147..=168 => CardinalDirection::SouthSouthEast,
            169..=191 => CardinalDirection::South,
            192..=213 => CardinalDirection::SouthSouthWest,
            214..=236 => CardinalDirection::SouthWest,
            237..=258 => CardinalDirection::WestSouthWest,
            259..=281 => CardinalDirection::West,
            282..=303 => CardinalDirection::WestNorthWest,
            304..=326 => CardinalDirection::NorthWest,
            327..=348 => CardinalDirection::NorthNorthWest,
            _ => CardinalDirection::Invalid,
        }
    }
}

impl fmt::Display for CardinalDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CardinalDirection::North => "n",
                CardinalDirection::NorthNorthEast => "nne",
                CardinalDirection::NorthEast => "ne",
                CardinalDirection::EastNorthEast => "ene",
                CardinalDirection::East => "e",
                CardinalDirection::EastSouthEast => "ese",
                CardinalDirection::SouthEast => "se",
                CardinalDirection::SouthSouthEast => "sse",
                CardinalDirection::South => "s",
                CardinalDirection::SouthSouthWest => "ssw",
                CardinalDirection::SouthWest => "sw",
                CardinalDirection::WestSouthWest => "wsw",
                CardinalDirection::West => "w",
                CardinalDirection::WestNorthWest => "wnw",
                CardinalDirection::NorthWest => "nw",
                CardinalDirection::NorthNorthWest => "nnw",
                CardinalDirection::Invalid => "",
            }
        )
    }
}

impl FromStr for CardinalDirection {
    type Err = DataParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "n" => Ok(CardinalDirection::North),
            "nne" => Ok(CardinalDirection::NorthNorthEast),
            "ne" => Ok(CardinalDirection::NorthEast),
            "ene" => Ok(CardinalDirection::EastNorthEast),
            "e" => Ok(CardinalDirection::East),
            "ese" => Ok(CardinalDirection::EastSouthEast),
            "se" => Ok(CardinalDirection::SouthEast),
            "sse" => Ok(CardinalDirection::SouthSouthEast),
            "s" => Ok(CardinalDirection::South),
            "ssw" => Ok(CardinalDirection::SouthSouthWest),
            "sw" => Ok(CardinalDirection::SouthWest),
            "wsw" => Ok(CardinalDirection::WestSouthWest),
            "w" => Ok(CardinalDirection::West),
            "wnw" => Ok(CardinalDirection::WestNorthWest),
            "nw" => Ok(CardinalDirection::NorthWest),
            "nnw" => Ok(CardinalDirection::NorthNorthWest),
            _ => Err(DataParseError::InvalidString),
        }
    }
}