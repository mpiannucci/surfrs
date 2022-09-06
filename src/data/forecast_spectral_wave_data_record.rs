use std::collections::VecDeque;
use std::f64;
use std::f64::consts::PI;
use std::iter::Skip;
use std::ops::Sub;
use std::str::{FromStr, Lines};

use chrono::{offset, DateTime, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::dimensional_data::DimensionalData;
use crate::location::Location;
use crate::swell::{Swell, SwellProvider, SwellProviderError, SwellSummary};
use crate::tools::analysis::{detect_peaks, watershed, WatershedError, watershed2};
use crate::tools::waves::{wavenu3, pt_mean};
use crate::units::{Direction, Measurement, UnitConvertible, Units};

use super::parseable_data_record::DataRecordParsingError;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
                        direction.push(Direction::from_degrees(value.to_degrees().round() as i32));
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForecastSpectralWaveDataRecord {
    pub date: DateTime<Utc>,
    pub location: Location,
    pub depth: DimensionalData<f64>,
    pub wind_speed: DimensionalData<f64>,
    pub wind_direction: DimensionalData<Direction>,
    pub current_speed: DimensionalData<f64>,
    pub current_direction: DimensionalData<Direction>,
    pub frequency: Vec<f64>,
    pub direction: Vec<Direction>,
    pub energy: Vec<f64>,
}

impl ForecastSpectralWaveDataRecord {
    // Fortan arrays
    // E(f, theta)
    // f is row
    // theta is columns
    // fortran stores in column major
    //      freq freq freq freq freq freq freq
    // dir  E    E    E    E    E    E    E
    // dir  E    E    E    E    E    E    E
    // dir  E    E    E    E    E    E    E
    //
    // So to get
    // E(2, 0) = 2
    // E(2, 3) = 11
    // E(2, 4) = 14
    pub fn dominant_spectra(&self) -> (Vec<f64>, Vec<Direction>, Vec<f64>) {
        let mut max_energies: Vec<f64> = Vec::with_capacity(self.frequency.len());
        let mut max_directions: Vec<Direction> = Vec::with_capacity(self.frequency.len());

        let mut spec_energy_sum: Vec<f64> = Vec::with_capacity(self.frequency.len());

        for i in 0..self.frequency.len() {
            let mut energy_sum = 0.0;
            let mut max_value = 0.0;
            let mut max_direction = Direction::from_degrees(0);
            for j in 0..self.direction.len() {
                let index = i + (self.frequency.len() * j);
                energy_sum += self.energy[index];

                if self.energy[index] > max_value {
                    max_value = self.energy[index];
                    max_direction = self.direction[j].clone();
                }
            }

            spec_energy_sum.push(energy_sum);
            max_directions.push(max_direction.invert());
            max_energies.push(max_value);
        }

        // for (_frequency_index, energy) in self.energy.chunks(self.direction.len()).enumerate() {
        //     let mut energy_sum = 0.0;
        //     let mut max_value = 0.0;
        //     let mut max_direction = Direction::from_degree(0);

        //     for (direction_index, value) in energy.iter().enumerate() {
        //         energy_sum += *value;
        //         if *value > max_value {
        //             max_value = *value;
        //             max_direction = self.direction[direction_index].clone();
        //         }
        //     }

        //     spec_energy_sum.push(energy_sum);
        //     max_energies.push(max_value);
        //     max_directions.push(max_direction);
        // }

        (self.frequency.clone(), max_directions, max_energies)
    }

    pub fn extract_partitions(&self) -> Result<usize, WatershedError> {
        let (imo, partition_count) = watershed2(
            &self.energy,
            self.frequency.len(),
            self.direction.len(),
            255,
        )?;

        // println!("-----");
        println!("{partition_count}");
        println!("{:?}", imo);

        let count = pt_mean(
            partition_count, 
            &imo, 
            &self.energy, 
            self.depth.value.unwrap(), 
            self.wind_speed.value.unwrap(), 
            self.wind_direction.value.as_ref().unwrap().radian(), 
            &self.frequency, 
            &self.direction
        );

        Ok(count)
    }
}

impl UnitConvertible<ForecastSpectralWaveDataRecord> for ForecastSpectralWaveDataRecord {
    fn to_units(&mut self, new_units: &Units) {
        self.depth.to_units(new_units);
        self.wind_speed.to_units(new_units);
        self.current_speed.to_units(new_units);
    }
}

impl SwellProvider for ForecastSpectralWaveDataRecord {
    fn swell_data(&self) -> Result<SwellSummary, crate::swell::SwellProviderError> {
        let (frequency, direction, energy) = self.dominant_spectra();

        let (minima_indexes, maxima_indexes) = detect_peaks(&energy, 0.05);

        let mut components = maxima_indexes
            .iter()
            .enumerate()
            .map(|(meta_index, i)| {
                let start = if meta_index == 0 {
                    0
                } else if i > &minima_indexes[meta_index - 1] {
                    minima_indexes[meta_index - 1]
                } else {
                    0
                };

                let end = if meta_index >= minima_indexes.len() {
                    energy.len()
                } else {
                    minima_indexes[meta_index]
                };

                Swell::from_spectra(
                    &frequency[start..end],
                    &energy[start..end],
                    &direction[start..end],
                )
            })
            .collect::<Result<Vec<_>, SwellProviderError>>()?;

        // Sort swell components from highest energy to lowest energy
        components.sort_by(|s1, s2| {
            s2.energy
                .clone()
                .unwrap()
                .value
                .unwrap()
                .total_cmp(&s1.energy.clone().unwrap().value.unwrap())
        });

        let dominant = components[0].clone();

        // See https://www.ndbc.noaa.gov/waveobs.shtml
        let wave_height = components
            .iter()
            .map(|c| c.wave_height.value.unwrap().powi(2))
            .sum::<f64>()
            .sqrt();

        Ok(SwellSummary {
            summary: Swell {
                wave_height: DimensionalData {
                    value: Some(wave_height),
                    measurement: dominant.wave_height.measurement,
                    unit: dominant.wave_height.unit,
                    variable_name: dominant.wave_height.variable_name,
                },
                period: dominant.period,
                direction: dominant.direction,
                energy: None,
            },
            components,
        })
    }
}

pub struct ForecastBulletinWaveRecordIterator<'a> {
    lines: Skip<Lines<'a>>,
    point_regex: Regex,
    metadata: ForecastSpectralWaveDataRecordMetadata,
}

impl<'a> ForecastBulletinWaveRecordIterator<'a> {
    pub fn from_data(data: &'a str) -> Result<Self, DataRecordParsingError> {
        let metadata = ForecastSpectralWaveDataRecordMetadata::from_str(data)?;
        let lines = data.lines().skip(metadata.line_count);

        let point_regex = Regex::new(".{0,12}\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)\\s*([+-]?[0-9]*[.]?[0-9]+)");
        let point_regex = point_regex.map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to create point regex: {}", e))
        })?;

        Ok(Self {
            lines,
            point_regex,
            metadata,
        })
    }

    fn parse_next(&mut self) -> Result<ForecastSpectralWaveDataRecord, DataRecordParsingError> {
        let line = self.lines.next().ok_or(DataRecordParsingError::EOF)?;

        // First line is the date
        let year = line[0..4].parse::<i32>().map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to parse year: {}", e))
        })?;

        let month = line[4..6].parse::<u32>().map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to parse month: {}", e))
        })?;

        let day = line[6..8].parse::<u32>().map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to parse day: {}", e))
        })?;

        let hour = line[9..11].parse::<u32>().map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to parse hour: {}", e))
        })?;

        let minute = line[11..13].parse::<u32>().map_err(|e| {
            DataRecordParsingError::ParseFailure(format!("Failed to parse minute: {}", e))
        })?;

        let date = Utc.ymd(year, month, day).and_hms(hour, minute, 0);

        let line = self.lines.next().ok_or(DataRecordParsingError::EOF)?;

        // Then the point data
        let (
            latitude,
            longitude,
            depth,
            wind_speed,
            wind_direction,
            current_speed,
            current_direction,
        ) = match self.point_regex.captures(line) {
            Some(captures) => Ok((
                captures
                    .get(1)
                    .ok_or(DataRecordParsingError::ParseFailure(
                        "Failed to parse latitude".into(),
                    ))?
                    .as_str()
                    .parse::<f64>()
                    .map_err(|e| {
                        DataRecordParsingError::ParseFailure(format!(
                            "Failed to parse latitude: {}",
                            e
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
                            "Failed to parse longitude: {}",
                            e
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
                            "Failed to parse depth: {}",
                            e
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
                            "Failed to parse wind speed: {}",
                            e
                        ))
                    })?,
                captures
                    .get(5)
                    .ok_or(DataRecordParsingError::ParseFailure(
                        "Failed to parse wind direction".into(),
                    ))?
                    .as_str()
                    .parse::<f64>()
                    .map_err(|e| {
                        DataRecordParsingError::ParseFailure(format!(
                            "Failed to parse wind direction: {}",
                            e
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
                            "Failed to parse current speed: {}",
                            e
                        ))
                    })?,
                captures
                    .get(7)
                    .ok_or(DataRecordParsingError::ParseFailure(
                        "Failed to parse current direction".into(),
                    ))?
                    .as_str()
                    .parse::<f64>()
                    .map_err(|e| {
                        DataRecordParsingError::ParseFailure(format!(
                            "Failed to parse current direction: {}",
                            e
                        ))
                    })?,
            )),
            None => {
                return Err(DataRecordParsingError::ParseFailure(
                    "Invalid data for point data".into(),
                ));
            }
        }?;

        // Then the frequency * direction data
        let energy_count = self.metadata.frequency.len() * self.metadata.direction.len();
        let mut raw_energy: Vec<f64> =
            Vec::with_capacity(self.metadata.frequency.len() * self.metadata.direction.len());

        while raw_energy.len() < energy_count {
            let line = self.lines.next().ok_or(DataRecordParsingError::EOF)?;

            line.split_whitespace().map(f64::from_str).for_each(|v| {
                if let Ok(v) = v {
                    raw_energy.push(v);
                }
            });
        }

        //let mut energy = vec![0.0; raw_energy.len()];
        //transpose::transpose(&raw_energy, &mut energy, self.metadata.direction.len(), self.metadata.frequency.len());

        Ok(ForecastSpectralWaveDataRecord {
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
                value: Some(Direction::from_degrees(wind_direction.round() as i32)),
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
                value: Some(Direction::from_degrees(current_direction.round() as i32)),
                variable_name: "current direction".into(),
                measurement: Measurement::Direction,
                unit: Units::Metric,
            },
            frequency: self.metadata.frequency.clone(),
            direction: self.metadata.direction.clone(),
            energy: raw_energy,
        })
    }
}

impl<'a> Iterator for ForecastBulletinWaveRecordIterator<'a> {
    type Item = Result<ForecastSpectralWaveDataRecord, DataRecordParsingError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_next() {
            Ok(v) => Some(Ok(v)),
            Err(e) => match e {
                DataRecordParsingError::EOF => None,
                _ => Some(Err(e)),
            },
        }
    }
}

pub struct ForecastSpectralWaveDataRecordCollection<'a> {
    data: &'a str,
}

impl<'a> ForecastSpectralWaveDataRecordCollection<'a> {
    pub fn from_data(data: &'a str) -> Self {
        ForecastSpectralWaveDataRecordCollection { data }
    }

    pub fn records(
        &'a mut self,
    ) -> Result<
        (
            ForecastSpectralWaveDataRecordMetadata,
            impl Iterator<Item = ForecastSpectralWaveDataRecord> + 'a,
        ),
        DataRecordParsingError,
    > {
        match ForecastBulletinWaveRecordIterator::from_data(self.data) {
            Ok(iter) => Ok((iter.metadata.clone(), iter.filter_map(|d| d.ok()))),
            Err(e) => Err(e),
        }
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

        assert_eq!(metadata.direction[0].degrees, 85);
        assert_eq!(metadata.direction[15].degrees, 295);
    }
}
