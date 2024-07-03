use chrono::prelude::*;
use itertools::izip;
use readap::DodsDataset;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SwdenWaveDataRecord {
    pub date: DateTime<Utc>,
    pub frequency: Vec<f64>,
    pub energy_spectra: Vec<f64>,
    pub mean_wave_direction: Vec<f64>,
    pub primary_wave_direction: Vec<f64>,
    pub first_polar_coefficient: Vec<f64>,
    pub second_polar_coefficient: Vec<f64>,
}

pub struct SwdenWaveDataRecordIterator<'a> {
    dates: Vec<DateTime<Utc>>,
    frequency: Vec<f64>,
    iter: Box<dyn Iterator<Item = (f64, f64, f64, f64, f64)> + 'a>,
    current: usize,
}

impl<'a> SwdenWaveDataRecordIterator<'a> {
    pub fn new(dataset: &'a DodsDataset) -> Self {
        let coords = dataset.variable_coords("spectral_wave_density").unwrap();
        let frequency: Vec<f64> = coords[1].1.clone().try_into().unwrap();

        let dates: Vec<i64> = coords[0].1.clone().try_into().unwrap();
        let dates = dates
            .iter()
            .map(|t| DateTime::from_timestamp(*t, 0).unwrap())
            .collect::<Vec<DateTime<Utc>>>();
        let energy_spectra = dataset.variable_data_iter("spectral_wave_density").unwrap();
        let mean_wave_direction = dataset.variable_data_iter("mean_wave_dir").unwrap();
        let primary_wave_direction = dataset.variable_data_iter("principal_wave_dir").unwrap();
        let first_polar_coefficient = dataset.variable_data_iter("wave_spectrum_r1").unwrap();
        let second_polar_coefficient = dataset.variable_data_iter("wave_spectrum_r2").unwrap();

        let iter = izip!(
            energy_spectra,
            mean_wave_direction,
            primary_wave_direction,
            first_polar_coefficient,
            second_polar_coefficient,
        )
        .map(|(e, m, p, f, s)| {
            (
                TryInto::<f64>::try_into(e).unwrap(),
                TryInto::<f64>::try_into(m).unwrap(),
                TryInto::<f64>::try_into(p).unwrap(),
                TryInto::<f64>::try_into(f).unwrap(),
                TryInto::<f64>::try_into(s).unwrap(),
            )
        });

        Self {
            dates,
            frequency,
            iter: Box::new(iter),
            current: 0,
        }
    }
}

impl<'a> Iterator for SwdenWaveDataRecordIterator<'a> {
    type Item = SwdenWaveDataRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let mut energy_spectra: Vec<f64> = vec![0.0; self.frequency.len()];
        let mut mean_wave_direction: Vec<f64> = vec![0.0; self.frequency.len()];
        let mut primary_wave_direction: Vec<f64> = vec![0.0; self.frequency.len()];
        let mut first_polar_coefficient: Vec<f64> = vec![0.0; self.frequency.len()];
        let mut second_polar_coefficient: Vec<f64> = vec![0.0; self.frequency.len()];

        for i in 0..self.frequency.len() {
            let (e, m, p, f, s) = self.iter.next()?;
            energy_spectra[i] = e;
            mean_wave_direction[i] = m;
            primary_wave_direction[i] = p;
            first_polar_coefficient[i] = f;
            second_polar_coefficient[i] = s;
        }

        let date = self.dates[self.current];
        let frequency = self.frequency.clone();
        self.current += 1;

        Some(SwdenWaveDataRecord {
            date,
            frequency,
            energy_spectra,
            mean_wave_direction,
            primary_wave_direction,
            first_polar_coefficient,
            second_polar_coefficient,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SwdenWaveDataRecordCollection<'a> {
    dataset: DodsDataset<'a>,
}

impl<'a> SwdenWaveDataRecordCollection<'a> {
    pub fn from_data(data: &'a [u8]) -> Self {
        let dataset = DodsDataset::from_bytes(&data).unwrap();

        Self { dataset }
    }

    pub fn records(&'a self) -> impl Iterator<Item = SwdenWaveDataRecord> + 'a {
        SwdenWaveDataRecordIterator::new(&self.dataset)
    }
}
