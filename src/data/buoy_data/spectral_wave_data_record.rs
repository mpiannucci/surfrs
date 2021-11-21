use crate::data::units::*;

use super::date_record::DateRecord;
use super::parseable_data_record::ParseableDataRecord;

#[derive(Clone, Debug)]
pub struct SpectralWaveDataRecord {
    pub date: DateRecord,
    pub separation_frequency: Option<f64>,
    pub value: Vec<f64>,
    pub frequency: Vec<f64>,
}

impl ParseableDataRecord for SpectralWaveDataRecord {
    fn from_data_row(row: &Vec<&str>) -> SpectralWaveDataRecord {
        let has_sep_freq: bool = row.len() % 2 == 0;
        let start_index: usize = match has_sep_freq {
            true => 6,
            false => 5,
        };
        let freq_count = (row.len() - start_index) / 2;

        let mut values: Vec<f64> = vec![0.0; freq_count];
        let mut freqs: Vec<f64> = vec![0.0; freq_count];

        for i in 0..freq_count {
            let index = start_index + i * 2;

            values[i] = row[index].parse().unwrap();
            freqs[i] = row[index+1].replace(&['(', ')'][..], "").parse().unwrap();
        }

        let separation_frequency = match has_sep_freq {
            true => Some(row[5].parse().unwrap_or(9.999)),
            false => None,
        };

        SpectralWaveDataRecord {
            date: DateRecord::from_data_row(row),
            separation_frequency: separation_frequency,
            value: values,
            frequency: freqs
        }
    }
}

impl UnitConvertible<SpectralWaveDataRecord> for SpectralWaveDataRecord {
    fn to_units(&mut self, new_units: &Units) {
        // TODO: Maybe some conversion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_wave_energy_data_row_parse() {
        let raw_data = "2018 09 01 10 00 9.999 0.000 (0.033) 0.000 (0.038) 0.000 (0.043) 0.000 (0.048) 0.000 (0.053) 0.000 (0.058) 0.000 (0.063) 0.021 (0.068) 0.021 (0.073) 0.074 (0.078) 0.085 (0.083) 0.074 (0.088) 0.085 (0.093) 0.085 (0.100) 0.148 (0.110) 0.138 (0.120) 0.074 (0.130) 0.244 (0.140) 0.392 (0.150) 0.477 (0.160) 0.572 (0.170) 1.060 (0.180) 0.339 (0.190) 0.382 (0.200) 0.265 (0.210) 0.265 (0.220) 0.318 (0.230) 0.329 (0.240) 0.329 (0.250) 0.350 (0.260) 0.244 (0.270) 0.371 (0.280) 0.180 (0.290) 0.180 (0.300) 0.170 (0.310) 0.117 (0.320) 0.127 (0.330) 0.095 (0.340) 0.064 (0.350) 0.085 (0.365) 0.085 (0.385) 0.074 (0.405) 0.021 (0.425) 0.011 (0.445) 0.021 (0.465) 0.011 (0.485)";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let spectral_data = SpectralWaveDataRecord::from_data_row(&data_row);
        
        assert!((spectral_data.separation_frequency.unwrap() - 9.999).abs() < 0.0001);
        assert_eq!(spectral_data.frequency.len(), 46);
        assert_eq!(spectral_data.value.len(), 46);
        assert!((spectral_data.frequency[2] - 0.043).abs() < 0.0001);
        assert!((spectral_data.value[9] - 0.074).abs() < 0.0001);
    }

    #[test]
    fn test_spectral_wave_directional_data_row_parse() {
        let raw_data = "2018 09 25 01 00 56.0 (0.025) 68.0 (0.030) 96.0 (0.035) 56.0 (0.040) 68.0 (0.045) 64.0 (0.050) 80.0 (0.055) 84.0 (0.060) 88.0 (0.065) 92.0 (0.070) 104.0 (0.075) 100.0 (0.080) 96.0 (0.085) 104.0 (0.090) 104.0 (0.095) 108.0 (0.101) 120.0 (0.110) 120.0 (0.120) 116.0 (0.130) 104.0 (0.140) 100.0 (0.150) 100.0 (0.160) 88.0 (0.170) 84.0 (0.180) 84.0 (0.190) 76.0 (0.200) 72.0 (0.210) 76.0 (0.220) 84.0 (0.230) 76.0 (0.240) 72.0 (0.250) 68.0 (0.260) 60.0 (0.270) 60.0 (0.280) 68.0 (0.290) 68.0 (0.300) 76.0 (0.310) 68.0 (0.320) 60.0 (0.330) 60.0 (0.340) 72.0 (0.350) 68.0 (0.360) 68.0 (0.370) 64.0 (0.380) 60.0 (0.390) 52.0 (0.400) 68.0 (0.410) 76.0 (0.420) 64.0 (0.430) 80.0 (0.440) 76.0 (0.450) 68.0 (0.460) 88.0 (0.470) 64.0 (0.480) 64.0 (0.490) 72.0 (0.500) 60.0 (0.510) 88.0 (0.520) 72.0 (0.530) 72.0 (0.540) 60.0 (0.550) 56.0 (0.560) 96.0 (0.570) 96.0 (0.580) ";
        let data_row: Vec<&str> = raw_data.split_whitespace().collect();

        let spectral_data = SpectralWaveDataRecord::from_data_row(&data_row);
        
        assert!(spectral_data.separation_frequency.is_none());
    }
}