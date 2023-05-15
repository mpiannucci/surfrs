use chrono::prelude::*;

use csv::Reader;
use serde::{Deserialize, Serialize};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Debug, Serialize, Deserialize)]
pub struct GribIndexRecord {
    pub index: usize,
    pub offset: usize,
    pub reference_date: DateTime<Utc>,
    pub var: String,
    pub level: String,
    pub valid: String,
}

impl ParseableDataRecord for GribIndexRecord {
    type Metadata = ();

    fn from_data_row(
        _metadata: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<Self, DataRecordParsingError>
    where
        Self: Sized,
    {
        let index: usize = row[0].parse().map_err(DataRecordParsingError::from)?;
        let offset: usize = row[1].parse().map_err(DataRecordParsingError::from)?;
        // HACK: Postfix with 00s to add minute stamp to allow NaiveDateTime to parse it
        let raw_date_string = format!("{raw}00", raw=row[2]);
        let var = row[3].to_string();
        let level = row[4].to_string();
        let valid = row[5].to_string();

        const DATE_FORMAT: &'static str = "d=%Y%m%d%H%M";
        let reference_date = NaiveDateTime::parse_from_str(&raw_date_string, DATE_FORMAT)
            .map_err(DataRecordParsingError::from)?;
        let reference_date = DateTime::<Utc>::from_utc(reference_date, Utc);

        Ok(GribIndexRecord {
            index,
            offset,
            reference_date,
            var,
            level,
            valid,
        })
    }
}

pub struct GribIndexRecordCollection<'a> {
    reader: Reader<&'a [u8]>,
}

impl<'a> GribIndexRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        let reader = csv::ReaderBuilder::new()
            .delimiter(b':')
            .trim(csv::Trim::All)
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .from_reader(data.as_bytes());

        GribIndexRecordCollection { reader }
    }

    pub fn records(&'a mut self) -> impl Iterator<Item = GribIndexRecord> + 'a {
        self.reader
            .records()
            .map(
                |result| -> Result<GribIndexRecord, DataRecordParsingError> {
                    if let Ok(record) = result {
                        let filtered_record: Vec<&str> =
                            record.iter().filter(|data| !data.is_empty()).collect();
                        let grib_index = GribIndexRecord::from_data_row(None, &filtered_record)?;
                        return Ok(grib_index);
                    }
                    Err(DataRecordParsingError::InvalidData)
                },
            )
            .filter_map(|d| d.ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grib_index_row_parse() {
        let raw_data = "8:2540945:d=2023051512:REFD:2 hybrid level:34 hour fcst:";
        let data_row: Vec<&str> = raw_data.split(":").collect();
        let grib_index = GribIndexRecord::from_data_row(None, &data_row).unwrap();

        let ref_date = Utc.with_ymd_and_hms(2023, 05, 15, 12, 0, 0).single().unwrap();
        assert_eq!(grib_index.index, 8);
        assert_eq!(grib_index.offset, 2540945);
        assert_eq!(grib_index.reference_date, ref_date);
        assert_eq!(grib_index.var, "REFD");
        assert_eq!(grib_index.level, "2 hybrid level");
        assert_eq!(grib_index.valid, "34 hour fcst");
    }
}