use std::str::FromStr;

use regex::Regex;

use crate::dimensional_data::DimensionalData;
use crate::location::Location;
use crate::units::{direction, Direction, Measurement, Units, UnitConvertible};

use super::date_record::DateRecord;
use super::parseable_data_record::{DataRecordParsingError, ParseableDataRecord};

#[derive(Clone, Debug)]
pub struct ForecastSpectralWaveDataRecordMetadata {
    pub frequency: Vec<f64>,
    pub direction: Vec<Direction>,
    pub point_count: usize,
    pub line_count: usize,
}

impl FromStr for ForecastSpectralWaveDataRecordMetadata {
    type Err = DataRecordParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let header_regex =
            Regex::new("'WAVEWATCH III SPECTRA'\\s*([0-9]{0,2})\\s*([0-9]{0,2})\\s*([0-9]{0,2})");
        let header_regex = header_regex.map_err(|e| {
            DataRecordParsingError::ParseFailure(format!(
                "Failed to create metadata header regex: {}",
                e
            ))
        })?;

        let mut line_count = 0;
        let mut lines = s.lines();

        let header_string = lines.next().ok_or(DataRecordParsingError::ParseFailure(
            "Invalid data for header metadata".into(),
        ))?;

        line_count += 1;

        let (frequency_count, direction_count, point_count) =
            match header_regex.captures(header_string) {
                Some(captures) => {
                    let frequency_count = captures
                        .get(1)
                        .ok_or(DataRecordParsingError::ParseFailure(
                            "Failed to parse frequency count".into(),
                        ))?
                        .as_str()
                        .parse::<usize>()
                        .map_err(|e| {
                            DataRecordParsingError::ParseFailure(format!(
                                "Failed to parse frequency count: {}",
                                e
                            ))
                        })?;

                    let direction_count = captures
                        .get(2)
                        .ok_or(DataRecordParsingError::ParseFailure(
                            "Failed to parse direction count".into(),
                        ))?
                        .as_str()
                        .parse::<usize>()
                        .map_err(|e| {
                            DataRecordParsingError::ParseFailure(format!(
                                "Failed to parse direction count: {}",
                                e
                            ))
                        })?;

                    let point_count = captures
                        .get(3)
                        .ok_or(DataRecordParsingError::ParseFailure(
                            "Failed to parse point count".into(),
                        ))?
                        .as_str()
                        .parse::<usize>()
                        .map_err(|e| {
                            DataRecordParsingError::ParseFailure(format!(
                                "Failed to parse point count: {}",
                                e
                            ))
                        })?;

                    Ok((frequency_count, direction_count, point_count))
                }
                None => {
                    return Err(DataRecordParsingError::ParseFailure(
                        "Invalid data for header metadata".into(),
                    ));
                }
            }?;

        let mut frequency: Vec<f64> = Vec::with_capacity(frequency_count);
        let mut direction: Vec<Direction> = Vec::with_capacity(direction_count);
        while frequency.len() < frequency_count {
            let data_line = lines
                .next()
                .ok_or(DataRecordParsingError::ParseFailure(
                    "Invalid data for frequency metadata".into(),
                ))?
                .split_whitespace();

            line_count += 1;

            data_line.for_each(|v| {
                if frequency.len() < frequency_count {
                    let value = v.parse::<f64>();
                    if let Ok(value) = value {
                        frequency.push(value);
                    }
                }
            })
        }

        while direction.len() < direction_count {
            let data_line = lines
                .next()
                .ok_or(DataRecordParsingError::ParseFailure(
                    "Invalid data for direction metadata".into(),
                ))?
                .split_whitespace();

            line_count += 1;

            data_line.for_each(|v| {
                if direction.len() < direction_count {
                    let value = v.parse::<f64>();
                    if let Ok(value) = value {
                        direction.push(Direction::from_degree(value.to_degrees().round() as i32));
                    }
                }
            })
        }

        Ok(ForecastSpectralWaveDataRecordMetadata {
            frequency,
            direction,
            point_count,
            line_count,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ForecastSpectralWaveDataRecord {
    pub date: DateRecord,
    pub location: Location,
    pub depth: DimensionalData<f64>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub current_speed: DimensionalData<f64>,
    pub current_direction: DimensionalData<Direction>,
    pub values: Vec<f64>,
}

impl ParseableDataRecord for ForecastSpectralWaveDataRecord {
    type Metadata = ForecastSpectralWaveDataRecordMetadata;

    fn from_data(
        data: &str,
        count: Option<usize>,
    ) -> Result<(Option<Self::Metadata>, Vec<ForecastSpectralWaveDataRecord>), DataRecordParsingError>
    {
        let metadata = ForecastSpectralWaveDataRecordMetadata::from_str(data)?;

        let mut lines = data.lines().skip(metadata.line_count);

        let mut records: Vec<ForecastSpectralWaveDataRecord> = Vec::new();

        let point_regex = Regex::new(".{0,12}\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)");
        let point_regex = point_regex.map_err(|e| {
            DataRecordParsingError::ParseFailure(format!(
                "Failed to create point regex: {}", e
            ))
        })?;

        while let Some(line) = lines.next() {
            // First line is the date
            let year = line[0..4].parse::<i32>().map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "Failed to parse year: {}",
                    e
                ))
            })?;

            let month = line[4..6].parse::<i32>().map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "Failed to parse year: {}",
                    e
                ))
            })?;

            let day = line[6..8].parse::<i32>().map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "Failed to parse year: {}",
                    e
                ))
            })?;

            let hour = line[11..13].parse::<i32>().map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "Failed to parse year: {}",
                    e
                ))
            })?;

            let minute = line[13..15].parse::<i32>().map_err(|e| {
                DataRecordParsingError::ParseFailure(format!(
                    "Failed to parse year: {}",
                    e
                ))
            })?; 

            let date = DateRecord{year, month, day, hour, minute};

            let line = lines.next().ok_or(DataRecordParsingError::ParseFailure(
                "Failed to parse point data".into(),
            ))?;

            // Then the point data
            let (latitude, longitude, depth, wind_speed, wind_direction, current_speed, current_direction) = match point_regex
                .captures(line) {
                    Some(captures) => {
                        Ok((
                            captures
                                .get(1)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse latitude".into(),
                                ))?
                                .as_str()
                                .parse::<f64>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse latitude: {}", e
                                ))
                            })?, 
                            captures
                                .get(2)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse longitude".into(),
                                ))?
                                .as_str()
                                .parse::<f64>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse longitude: {}", e
                                ))
                            })?, 
                            captures
                                .get(3)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse depth".into(),
                                ))?
                                .as_str()
                                .parse::<f64>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse depth: {}", e
                                ))
                            })?, 
                            captures
                                .get(4)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse wind speed".into(),
                                ))?
                                .as_str()
                                .parse::<f64>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse wind speed: {}", e
                                ))
                            })?, 
                            captures
                                .get(5)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse wind direction".into(),
                                ))?
                                .as_str()
                                .parse::<i32>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse wind direction: {}", e
                                ))
                            })?, 
                            captures
                                .get(6)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse current speed".into(),
                                ))?
                                .as_str()
                                .parse::<f64>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse current speed: {}", e
                                ))
                            })?, 
                            captures
                                .get(7)
                                .ok_or(DataRecordParsingError::ParseFailure(
                                    "Failed to parse current direction".into(),
                                ))?
                                .as_str()
                                .parse::<i32>()
                                .map_err(|e| {
                                    DataRecordParsingError::ParseFailure(format!(
                                    "Failed to parse current direction: {}", e
                                ))
                            })?, 
                        ))
                    }, 
                    None => {
                        return Err(DataRecordParsingError::ParseFailure(
                            "Invalid data for point data".into(),
                        ));
                    }
                }?;

            // Then the frequency * direction data
            let energy_count = metadata.frequency.len() * metadata.direction.len();
            let mut values: Vec<f64> = Vec::with_capacity(metadata.frequency.len() * metadata.direction.len());

            while values.len() < energy_count {
                let line = lines.next().ok_or(DataRecordParsingError::ParseFailure(
                    "Failed to parse energy data".into(),
                ))?;

                line
                    .split_whitespace()
                    .map(f64::from_str)
                    .for_each(|v| {
                        if let Ok(v) = v {
                            values.push(v);
                        }
                    });
            }

            records.push(ForecastSpectralWaveDataRecord{
                date,
                location: Location::new(latitude, longitude, "".into()), 
                depth: DimensionalData {
                    value: Some(depth), 
                    variable_name: "depth".into(),
                    measurement: Measurement::Length, 
                    unit: Units::Metric,
                },
                wind_speed: DimensionalData {
                    value: Some(wind_speed), 
                    variable_name: "wind speed".into(),
                    measurement: Measurement::Speed, 
                    unit: Units::Metric,
                },
                wind_direction: DimensionalData {
                    value: Some(Direction::from_degree(wind_direction)), 
                    variable_name: "wind direction".into(),
                    measurement: Measurement::Direction, 
                    unit: Units::Metric,
                },
                current_speed: DimensionalData {
                    value: Some(current_speed), 
                    variable_name: "current speed".into(),
                    measurement: Measurement::Speed, 
                    unit: Units::Metric,
                },
                current_direction: DimensionalData {
                    value: Some(Direction::from_degree(current_direction)), 
                    variable_name: "current direction".into(),
                    measurement: Measurement::Direction, 
                    unit: Units::Metric,
                },
                values,
            });
        };

        Ok((Some(metadata), records))
    }
}

impl UnitConvertible<ForecastSpectralWaveDataRecord> for ForecastSpectralWaveDataRecord {
    fn to_units(&mut self, new_units: &Units) {
        self.depth.to_units(new_units);
        self.wind_speed.to_units(new_units);
        self.current_speed.to_units(new_units);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forecast_spectra_metadata() {
        let metadata = "'WAVEWATCH III SPECTRA'     50    36     1 'spectral resolution for points'
        0.350E-01 0.375E-01 0.401E-01 0.429E-01 0.459E-01 0.491E-01 0.525E-01 0.562E-01
        0.601E-01 0.643E-01 0.689E-01 0.737E-01 0.788E-01 0.843E-01 0.902E-01 0.966E-01
        0.103E+00 0.111E+00 0.118E+00 0.127E+00 0.135E+00 0.145E+00 0.155E+00 0.166E+00
        0.178E+00 0.190E+00 0.203E+00 0.217E+00 0.233E+00 0.249E+00 0.266E+00 0.285E+00
        0.305E+00 0.326E+00 0.349E+00 0.374E+00 0.400E+00 0.428E+00 0.458E+00 0.490E+00
        0.524E+00 0.561E+00 0.600E+00 0.642E+00 0.687E+00 0.735E+00 0.787E+00 0.842E+00
        0.901E+00 0.964E+00
         0.148E+01  0.131E+01  0.113E+01  0.960E+00  0.785E+00  0.611E+00  0.436E+00
         0.262E+00  0.873E-01  0.620E+01  0.602E+01  0.585E+01  0.567E+01  0.550E+01
         0.532E+01  0.515E+01  0.497E+01  0.480E+01  0.463E+01  0.445E+01  0.428E+01
         0.410E+01  0.393E+01  0.375E+01  0.358E+01  0.340E+01  0.323E+01  0.305E+01
         0.288E+01  0.271E+01  0.253E+01  0.236E+01  0.218E+01  0.201E+01  0.183E+01
         0.166E+01";

        let metadata = ForecastSpectralWaveDataRecordMetadata::from_str(metadata).unwrap();

        assert_eq!(metadata.frequency.len(), 50);
        assert_eq!(metadata.direction.len(), 36);
        assert_eq!(metadata.point_count, 1);
        assert_eq!(metadata.line_count, 14);

        assert_eq!(metadata.frequency[0], 0.035);
        assert_eq!(metadata.frequency[11], 0.0737);

        assert_eq!(metadata.direction[0].degree.unwrap(), 85);
        assert_eq!(metadata.direction[15].degree.unwrap(), 295);
    }
}
