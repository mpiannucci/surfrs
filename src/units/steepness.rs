use std::fmt;
use std::str::FromStr;
use super::DataParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum Steepness {
    VerySteep,
    Steep,
    Average,
    Swell,
    NA,
}

impl Steepness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Steepness::VerySteep => "VERY_STEEP",
            Steepness::Steep => "STEEP",
            Steepness::Average => "AVERAGE",
            Steepness::Swell => "SWELL",
            Steepness::NA => "NA",
        }
    }
}

impl FromStr for Steepness {
    type Err = DataParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VERY_STEEP" => Ok(Steepness::VerySteep),
            "STEEP" => Ok(Steepness::Steep),
            "Average" => Ok(Steepness::Average),
            "SWELL" => Ok(Steepness::Swell),
            "NA" => Ok(Steepness::NA),
            _ => Err(DataParseError::InvalidString),
        }
    }
}

impl fmt::Display for Steepness {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}