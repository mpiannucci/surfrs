
#[derive(Clone, Debug)]
pub enum DataRecordParsingError {
    NotImplemented, 
    InvalidData, 
    ParseFailure(String),
}

pub trait ParseableDataRecord {
    type Metadata;

    fn from_data(raw_data: &str, count: Option<usize>) -> Result<(Option<Self::Metadata>, Vec<Self>), DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
    fn from_data_row(metadata: &Option<Self::Metadata>, row: &Vec<&str>) -> Result<Self, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}
