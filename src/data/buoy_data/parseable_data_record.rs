
pub trait ParseableDataRecord {
    fn from_data_row(row: &Vec<&str>) -> Self;
}
