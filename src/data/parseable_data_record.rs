use std::{
    format,
    num::{ParseFloatError, ParseIntError},
};

use chrono::ParseError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataRecordParsingError {
    EOF,
    NotImplemented,
    InvalidData,
    ParseFailure(String),
    KeyMissing(String),
}

impl std::fmt::Display for DataRecordParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataRecordParsingError::EOF => write!(f, "EOF while parsing data"),
            DataRecordParsingError::NotImplemented => {
                write!(f, "Encountered not implemented behavior")
            }
            DataRecordParsingError::InvalidData => {
                write!(f, "Invalid data encountered while parsing data")
            }
            DataRecordParsingError::ParseFailure(e) => write!(f, "Data parse failure: {e}"),
            DataRecordParsingError::KeyMissing(key) => {
                write!(f, "Key missing from data: {}", key)
            }
        }
    }
}

impl From<ParseFloatError> for DataRecordParsingError {
    fn from(e: ParseFloatError) -> Self {
        DataRecordParsingError::ParseFailure(format!("Float: {e}"))
    }
}

impl From<ParseIntError> for DataRecordParsingError {
    fn from(e: ParseIntError) -> Self {
        DataRecordParsingError::ParseFailure(format!("Int: {e}"))
    }
}

impl From<ParseError> for DataRecordParsingError {
    fn from(e: ParseError) -> Self {
        DataRecordParsingError::ParseFailure(format!("Date: {e}"))
    }
}

pub trait ParseableDataRecord {
    type Metadata;

    fn from_data_row(
        _metadata: Option<&Self::Metadata>,
        _row: &Vec<&str>,
    ) -> Result<Self, DataRecordParsingError>
    where
        Self: Sized,
    {
        Err(DataRecordParsingError::NotImplemented)
    }
}
