//! Parse SWAN `BLOCK ... HEAD` ASCII raster output (e.g. `hsign.blk`).
//!
//! SWAN prints the computational grid in bands of columns: each band opens
//! with a `%    0   1   2 ...` x-index header, followed by one fixed-width
//! row per y index (4-character cells starting at column 6, `****` for
//! exception/land cells), all scaled by the `Unit:` factor from the file
//! header.

use std::{error::Error, fmt};

#[derive(Clone, Debug)]
pub struct SwanBlockFile {
    /// (x_index, y_index) -> value, populated cells only, already scaled
    /// by the unit factor.
    pub values: Vec<((usize, usize), f64)>,
    /// The `Unit:` scale factor from the header.
    pub unit: f64,
}

impl SwanBlockFile {
    pub fn from_data(data: &str) -> Result<Self, SwanBlockError> {
        let mut values = Vec::new();
        let mut x_indices: Vec<usize> = Vec::new();
        let mut unit = 1.0;

        for (index, line) in data.lines().enumerate() {
            let line_number = index + 1;
            if let Some(rest) = line.split("Unit:").nth(1) {
                unit = rest
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| SwanBlockError::parse(line_number, "Unit: with no value"))?
                    .parse()
                    .map_err(|error| {
                        SwanBlockError::parse(line_number, format!("invalid Unit: {error}"))
                    })?;
            } else if line.starts_with("%   ") && line.bytes().any(|b| b.is_ascii_digit()) {
                x_indices = line
                    .split_whitespace()
                    .skip(1)
                    .map(|token| {
                        token.parse().map_err(|error| {
                            SwanBlockError::parse(line_number, format!("invalid x index: {error}"))
                        })
                    })
                    .collect::<Result<_, _>>()?;
            } else if !x_indices.is_empty() && is_data_row(line) {
                let y_index: usize = line[..5.min(line.len())].trim().parse().map_err(|error| {
                    SwanBlockError::parse(line_number, format!("invalid y index: {error}"))
                })?;
                let bytes = line.as_bytes();
                let cells = (6..line.len())
                    .step_by(4)
                    .map(|start| &bytes[start..line.len().min(start + 4)]);
                for (&x_index, cell) in x_indices.iter().zip(cells) {
                    let cell = std::str::from_utf8(cell)
                        .map_err(|_| SwanBlockError::parse(line_number, "non-ASCII cell"))?;
                    if !cell.contains('*') && !cell.trim().is_empty() {
                        let value: f64 = cell.trim().parse().map_err(|error| {
                            SwanBlockError::parse(line_number, format!("invalid cell: {error}"))
                        })?;
                        values.push(((x_index, y_index), value * unit));
                    }
                }
            }
        }

        if values.is_empty() {
            return Err(SwanBlockError::Empty);
        }
        Ok(SwanBlockFile { values, unit })
    }

    /// Dense row-major (y, x) grid with NaN for missing cells, y ascending.
    pub fn dense(&self, width: usize, height: usize) -> Vec<f64> {
        let mut field = vec![f64::NAN; width * height];
        for &((x, y), value) in &self.values {
            if x < width && y < height {
                field[y * width + x] = value;
            }
        }
        field
    }
}

/// Optional leading whitespace, then a y-index integer, then a whitespace
/// character (`^\s*\d+\s`).
fn is_data_row(line: &str) -> bool {
    let rest = line.trim_start();
    let digits = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    digits > 0
        && rest[digits..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
}

#[derive(Debug)]
pub enum SwanBlockError {
    Parse { line: usize, message: String },
    Empty,
}

impl SwanBlockError {
    fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for SwanBlockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { line, message } => {
                write!(formatter, "SWAN block parse error on line {line}: {message}")
            }
            Self::Empty => write!(formatter, "no grid values found in SWAN block output"),
        }
    }
}

impl Error for SwanBlockError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mimics real SWAN BLOCK layout: header comments with the Unit factor,
    /// then two column bands (SWAN wraps wide grids), star cells for land.
    /// Raw string: the 5-char y label + 4-char cells are position-sensitive.
    const EXAMPLE: &str = r"%
%
% Run:01    Frame:  COMPGRID **  Significant wave height                 , Unit:  0.1000E-01 m
%
%         X --->
%
%     0   1   2
%Y
    2 ****  12  34
    1  100 101 ***
    0   50  51  52
%     3   4
%Y
    2  77 ****
    1  88  89
    0  90  91
";

    #[test]
    fn parses_bands_stars_and_unit_scaling() {
        let block = SwanBlockFile::from_data(EXAMPLE).unwrap();
        assert_eq!(block.unit, 0.01);
        // 15 cells minus 3 starred
        assert_eq!(block.values.len(), 12);

        let dense = block.dense(5, 3);
        assert!((dense[0] - 0.50).abs() < 1e-12); // (0,0)
        assert!((dense[2 * 5 + 1] - 0.12).abs() < 1e-12); // (1,2)
        assert!(dense[2 * 5].is_nan()); // (0,2) starred
        assert!((dense[1 * 5 + 3] - 0.88).abs() < 1e-12); // (3,1) second band
        assert!(dense[2 * 5 + 4].is_nan()); // (4,2) starred in second band
        assert!((dense[0 * 5 + 4] - 0.91).abs() < 1e-12); // (4,0)
    }
}
