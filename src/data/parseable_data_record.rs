use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataRecordParsingError {
    EOF,
    NotImplemented, 
    InvalidData, 
    ParseFailure(String),
}

impl std::fmt::Display for DataRecordParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataRecordParsingError::EOF => write!(f, "EOF while parsing data"),
            DataRecordParsingError::NotImplemented => write!(f, "Encountered not implemented behavior"),
            DataRecordParsingError::InvalidData => write!(f, "Invalid data encountered while parsing data"),
            DataRecordParsingError::ParseFailure(_) => write!(f, "Data parse failure"),
        } 
    }
}

pub trait ParseableDataRecord {
    type Metadata;

    fn from_data_row(_metadata: Option<&Self::Metadata>, _row: &Vec<&str>) -> Result<Self, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}

