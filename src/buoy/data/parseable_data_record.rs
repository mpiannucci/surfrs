
#[derive(Clone, Debug)]
pub enum DataRecordParsingError {
    NotImplemented, 
    InvalidData, 
    ParseFailure(String),
}

pub trait ParseableDataRecord {
    type Metadata;

    fn from_data(_raw_data: &str, _count: Option<usize>) -> Result<(Option<Self::Metadata>, Vec<Self>), DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
    fn from_data_row(_metadata: Option<&Self::Metadata>, _row: &Vec<&str>) -> Result<Self, DataRecordParsingError> where Self: Sized {
        Err(DataRecordParsingError::NotImplemented)
    }
}
