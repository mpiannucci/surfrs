
#[derive(Clone, Debug)]
pub enum DataRecordParsingError {
    NotImplemented, 
    InvalidData
}

pub trait ParseableDataRecord {
    fn from_data(raw_data: &str) -> Result<Vec<Self>, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
    fn from_data_row(row: &Vec<&str>) -> Result<Self, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}
