use std::str::FromStr;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::dimensional_data::DimensionalData;
use crate::location::Location;
use crate::units::{Unit, UnitConvertible, UnitSystem};

use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GEFSBulletinWaveRecordMetadata {
    pub location: Location,
    pub model_run_date: DateTime<Utc>,
}

impl FromStr for GEFSBulletinWaveRecordMetadata {
    type Err = DataRecordParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let location_parser = Regex::new(
            r"Location\s*:\s*(.{0,10}?)\s*\(\s*([+-]?[0-9]*\.?[0-9]+[NS])\s+([+-]?[0-9]*\.?[0-9]+[EW])\)",
        )
        .map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to create location regex: {e}"))
        })?;

        // Locate header lines by keyword rather than by position: the real
        // bulletin files (extracted from the station tar) start with a leading
        // blank line, so we cannot assume the Location line comes first.
        let location_str = s
            .lines()
            .find(|l| l.contains("Location"))
            .ok_or(DataRecordParsingError::ParseFailure(
                "Invalid data for location metadata".into(),
            ))?;

        let location = match location_parser.captures(location_str) {
            Some(captures) => {
                let name = captures
                    .get(1)
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Missing location name capture".into())
                    })?
                    .as_str()
                    .trim();
                let latitude = parse_lat(
                    captures
                        .get(2)
                        .ok_or_else(|| {
                            DataRecordParsingError::ParseFailure("Missing latitude capture".into())
                        })?
                        .as_str(),
                )?;
                let longitude = parse_lon(
                    captures
                        .get(3)
                        .ok_or_else(|| {
                            DataRecordParsingError::ParseFailure("Missing longitude capture".into())
                        })?
                        .as_str(),
                )?;
                Ok(Location::new(latitude, longitude, name.into()))
            }
            None => Err(DataRecordParsingError::ParseFailure(
                "Failed to capture location data from regex".into(),
            )),
        }?;

        // Cycle line: "Cycle    : 20260305 t12z UTC"
        let cycle_parser = Regex::new(r"Cycle\s*:\s*([0-9]{4})([0-9]{2})([0-9]{2})\s+t([0-9]{2})z")
            .map_err(|e| {
                DataRecordParsingError::ParseFailure(format!("Failed to create cycle regex: {e}"))
            })?;

        let cycle_line = s
            .lines()
            .find(|l| l.contains("Cycle"))
            .ok_or_else(|| DataRecordParsingError::ParseFailure("Missing cycle line".into()))?;

        let model_run_date = match cycle_parser.captures(cycle_line) {
            Some(captures) => {
                let year = captures
                    .get(1)
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Missing cycle year capture".into())
                    })?
                    .as_str()
                    .parse::<i32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure("Failed to parse cycle year".into())
                    })?;
                let month = captures
                    .get(2)
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Missing cycle month capture".into())
                    })?
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure("Failed to parse cycle month".into())
                    })?;
                let day = captures
                    .get(3)
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Missing cycle day capture".into())
                    })?
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure("Failed to parse cycle day".into())
                    })?;
                let hour = captures
                    .get(4)
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Missing cycle hour capture".into())
                    })?
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| {
                        DataRecordParsingError::ParseFailure("Failed to parse cycle hour".into())
                    })?;

                Utc.with_ymd_and_hms(year, month, day, hour, 0, 0)
                    .single()
                    .ok_or_else(|| {
                        DataRecordParsingError::ParseFailure("Invalid cycle date/time".into())
                    })
            }
            None => Err(DataRecordParsingError::ParseFailure(
                "Failed to match cycle line".into(),
            )),
        }?;

        Ok(GEFSBulletinWaveRecordMetadata {
            location,
            model_run_date,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GEFSBulletinWaveRecord {
    pub date: DateTime<Utc>,
    pub significant_wave_height_avg: DimensionalData<f64>,
    pub significant_wave_height_spread: DimensionalData<f64>,
    pub peak_period_avg: DimensionalData<f64>,
    pub peak_period_spread: DimensionalData<f64>,
    pub wind_speed_avg: DimensionalData<f64>,
    pub wind_speed_spread: DimensionalData<f64>,
    pub prob_hs_gt_1m: f64,
    pub prob_hs_gt_2m: f64,
    pub prob_hs_gt_3m: f64,
    pub prob_hs_gt_5_5m: f64,
    pub prob_hs_gt_7m: f64,
    pub prob_hs_gt_9m: f64,
}

impl ParseableDataRecord for GEFSBulletinWaveRecord {
    type Metadata = GEFSBulletinWaveRecordMetadata;

    fn from_data_row(
        metadata: Option<&Self::Metadata>,
        row: &Vec<&str>,
    ) -> Result<Self, DataRecordParsingError>
    where
        Self: Sized,
    {
        if row.len() < 13 {
            return Err(DataRecordParsingError::ParseFailure(format!(
                "Expected 13 columns, got {}",
                row.len()
            )));
        }

        // row[0] = "05 12" (day hour)
        let day_hour: Vec<&str> = row[0].split_whitespace().collect();
        if day_hour.len() < 2 {
            return Err(DataRecordParsingError::ParseFailure(
                "Failed to parse day/hour column".into(),
            ));
        }
        let day = day_hour[0]
            .parse::<u32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse day".into()))?;
        let hour = day_hour[1]
            .parse::<u32>()
            .map_err(|_| DataRecordParsingError::ParseFailure("Failed to parse hour".into()))?;

        let model_date = match metadata {
            Some(m) => Ok(m.model_run_date.date_naive()),
            None => Err(DataRecordParsingError::InvalidData),
        }?;

        let month = if model_date.day() > day {
            model_date.month() + 1
        } else {
            model_date.month()
        };

        let date = Utc
            .with_ymd_and_hms(model_date.year(), month, day, hour, 0, 0)
            .single()
            .ok_or_else(|| {
                DataRecordParsingError::ParseFailure("Invalid forecast date/time".into())
            })?;

        let parse_f64 = |s: &str| -> Result<f64, DataRecordParsingError> {
            s.trim()
                .parse::<f64>()
                .map_err(|e| DataRecordParsingError::ParseFailure(format!("Float parse: {e}")))
        };

        Ok(GEFSBulletinWaveRecord {
            date,
            significant_wave_height_avg: DimensionalData::from_raw_data(
                row[1].trim(),
                "significant wave height avg".into(),
                Unit::Meters,
            ),
            significant_wave_height_spread: DimensionalData::from_raw_data(
                row[2].trim(),
                "significant wave height spread".into(),
                Unit::Meters,
            ),
            peak_period_avg: DimensionalData::from_raw_data(
                row[3].trim(),
                "peak period avg".into(),
                Unit::Seconds,
            ),
            peak_period_spread: DimensionalData::from_raw_data(
                row[4].trim(),
                "peak period spread".into(),
                Unit::Seconds,
            ),
            wind_speed_avg: DimensionalData::from_raw_data(
                row[5].trim(),
                "wind speed avg".into(),
                Unit::MetersPerSecond,
            ),
            wind_speed_spread: DimensionalData::from_raw_data(
                row[6].trim(),
                "wind speed spread".into(),
                Unit::MetersPerSecond,
            ),
            prob_hs_gt_1m: parse_f64(row[7])?,
            prob_hs_gt_2m: parse_f64(row[8])?,
            prob_hs_gt_3m: parse_f64(row[9])?,
            prob_hs_gt_5_5m: parse_f64(row[10])?,
            prob_hs_gt_7m: parse_f64(row[11])?,
            prob_hs_gt_9m: parse_f64(row[12])?,
        })
    }
}

impl UnitConvertible for GEFSBulletinWaveRecord {
    fn to_units(&mut self, new_units: &UnitSystem) -> &mut Self {
        self.significant_wave_height_avg.to_units(new_units);
        self.significant_wave_height_spread.to_units(new_units);
        self.wind_speed_avg.to_units(new_units);
        self.wind_speed_spread.to_units(new_units);
        self
    }
}

pub struct GEFSBulletinWaveRecordCollection<'a> {
    data: &'a str,
}

impl<'a> GEFSBulletinWaveRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        GEFSBulletinWaveRecordCollection { data }
    }

    pub fn records(
        &'a self,
    ) -> Result<
        (
            GEFSBulletinWaveRecordMetadata,
            impl Iterator<Item = GEFSBulletinWaveRecord> + 'a,
        ),
        DataRecordParsingError,
    > {
        let metadata = self.data.parse::<GEFSBulletinWaveRecordMetadata>()?;
        let metadata_clone = metadata.clone();

        let records = self
            .data
            .lines()
            .filter(|line| is_data_row(line))
            .filter_map(move |line| {
                let cols: Vec<&str> = line
                    .split('|')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                GEFSBulletinWaveRecord::from_data_row(Some(&metadata), &cols).ok()
            });

        Ok((metadata_clone, records))
    }
}

/// Returns true for pipe-delimited data rows (not separators or header rows).
fn is_data_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    let first_cell = trimmed.split('|').nth(1).unwrap_or("").trim();
    !first_cell.is_empty()
        && !first_cell.contains("day")
        && !first_cell.contains("hour")
        && first_cell
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
}

fn parse_lat(raw: &str) -> Result<f64, DataRecordParsingError> {
    let trimmed = raw.trim();
    let num_str = trimmed.trim_end_matches(|c: char| c.is_alphabetic());
    let value = num_str.parse::<f64>().map_err(|e| {
        DataRecordParsingError::ParseFailure(format!("Failed to parse latitude: {e}"))
    })?;
    Ok(if trimmed.ends_with('S') {
        -value.abs()
    } else {
        value.abs()
    })
}

fn parse_lon(raw: &str) -> Result<f64, DataRecordParsingError> {
    let trimmed = raw.trim();
    let num_str = trimmed.trim_end_matches(|c: char| c.is_alphabetic());
    let value = num_str.parse::<f64>().map_err(|e| {
        DataRecordParsingError::ParseFailure(format!("Failed to parse longitude: {e}"))
    })?;
    Ok(if trimmed.ends_with('W') {
        -value.abs()
    } else {
        value.abs()
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    const SAMPLE: &str = r#" Location : 44097      (40.969N  -71.127W)
 Model    : NCEP Global Wave Ensemble System (gefs.wave)
 Cycle    : 20260305 t12z UTC

+-------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
| day   | Hs avg | Hs spr | Tp avg | Tp spr | U10avg | U10spr | P(Hs>) | P(Hs>) | P(Hs>) | P(Hs>) | P(Hs>) | P(Hs>) |
|  hour |  (m)   |  (m)   |  (s)   |  (m)   |  (m/s) |  (m/s) |  1.00m |  2.00m | 3.00m  |  5.50m |  7.00m |  9.00m |
+-------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
| 05 12 |  1.05  |  0.03  |  7.14  |  0.25  |  1.16  |  0.56  |  1.00  |  0.98  |  0.00  |  0.00  |  0.00  |  0.00  |
| 06 00 |  1.10  |  0.06  |  9.03  |  0.92  |  8.71  |  0.93  |  1.00  |  0.94  |  0.00  |  0.00  |  0.00  |  0.00  |
+-------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+--------+
"#;

    #[test]
    fn test_parse_gefs_bulletin() {
        let meta = SAMPLE.parse::<GEFSBulletinWaveRecordMetadata>().unwrap();
        assert_eq!(meta.location.name, "44097");
        assert!((meta.location.latitude - 40.969).abs() < 0.001);
        assert!((meta.location.longitude - (-71.127)).abs() < 0.001);
        assert_eq!(meta.model_run_date.year(), 2026);
        assert_eq!(meta.model_run_date.month(), 3);
        assert_eq!(meta.model_run_date.day(), 5);
        assert_eq!(meta.model_run_date.hour(), 12);

        let collection = GEFSBulletinWaveRecordCollection::from_data(SAMPLE);
        let (_, records) = collection.records().unwrap();
        let rows: Vec<_> = records.collect();

        assert_eq!(rows.len(), 2);

        let first = &rows[0];
        assert_eq!(first.date.month(), 3);
        assert_eq!(first.date.day(), 5);
        assert_eq!(first.date.hour(), 12);
        assert!((first.significant_wave_height_avg.value.unwrap() - 1.05).abs() < 0.001);
        assert!((first.significant_wave_height_spread.value.unwrap() - 0.03).abs() < 0.001);
        assert!((first.peak_period_avg.value.unwrap() - 7.14).abs() < 0.001);
        assert!((first.wind_speed_avg.value.unwrap() - 1.16).abs() < 0.001);
        assert!((first.prob_hs_gt_1m - 1.00).abs() < 0.001);
        assert!((first.prob_hs_gt_2m - 0.98).abs() < 0.001);
        assert!((first.prob_hs_gt_3m - 0.00).abs() < 0.001);
    }

    #[test]
    fn test_parse_gefs_bulletin_leading_blank_line() {
        // Real .bull files extracted from the station tar begin with a blank
        // line before the Location header. The parser must tolerate it.
        let data = format!("\n{SAMPLE}");

        let meta = data.parse::<GEFSBulletinWaveRecordMetadata>().unwrap();
        assert_eq!(meta.location.name, "44097");
        assert_eq!(meta.model_run_date.hour(), 12);

        let collection = GEFSBulletinWaveRecordCollection::from_data(&data);
        let (_, records) = collection.records().unwrap();
        assert_eq!(records.count(), 2);
    }
}
