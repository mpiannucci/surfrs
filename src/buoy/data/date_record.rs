use super::parseable_data_record::{ParseableDataRecord, DataRecordParsingError};

#[derive(Clone, Debug)]
pub struct DateRecord {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
}

impl ParseableDataRecord for DateRecord {
    type Metadata = ();

    fn from_data_row(metadata: &Option<Self::Metadata>, row: &Vec<&str>) -> Result<DateRecord, DataRecordParsingError> {
        Ok(DateRecord {
            year: row[0].clone().parse().unwrap(),
            month: row[1].clone().parse().unwrap(),
            day: row[2].clone().parse().unwrap(),
            hour: row[3].clone().parse().unwrap(),
            minute: row[4].clone().parse().unwrap(),
        })
    }
}