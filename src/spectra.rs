use ndarray::prelude::*;

use crate::tools::vector::diff;


pub struct Spectra {
    pub frequency: Vec<f64>, 
    pub direction: Vec<f64>, 
    pub values: Vec<f64>,
}

impl Spectra {
    pub fn new(frequency: Vec<f64>, direction: Vec<f64>, values: Vec<f64>) -> Self {
        Spectra {
            frequency, 
            direction, 
            values,
        }
    }

    pub fn nk(&self) -> usize {
        self.frequency.len()
    }

    pub fn dk(&self) -> Vec<f64> {
        diff(&self.frequency)
    }

    pub fn nth(&self) -> usize {
        self.direction.len()
    }

    pub fn dth(&self) -> Vec<f64> {
        diff(&self.direction)
    }

    pub fn oned(&self) -> Vec<f64> {
        let dth = self.dth();
        let nk = self.nk();
        let nth = self.nth();

        let mut oned = vec![0.0; nk];
        for ik in 0..nk {
            for ith in 0..nth {
                let i = ik + (ith * nk);
                oned[ik] += dth[ith] * self.values[i];
            }
        }

        oned
    }
}