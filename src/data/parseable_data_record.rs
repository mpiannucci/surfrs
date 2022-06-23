
#[derive(Clone, Debug)]
pub enum DataRecordParsingError {
    EOF,
    NotImplemented, 
    InvalidData, 
    ParseFailure(String),
}

pub trait ParseableDataRecord {
    type Metadata;

    fn from_data_row(_metadata: Option<&Self::Metadata>, _row: &Vec<&str>) -> Result<Self, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}
