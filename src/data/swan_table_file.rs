//! Parse SWAN `TABLE ... HEAD` ASCII point output (e.g. `matunuck.tbl`).
//!
//! The HEAD format prints comment lines (`%`) including one row of column
//! names and one row of bracketed units, then one whitespace-separated data
//! row per output time:
//!
//! ```text
//! %
//! % Run:01    Table:SPT00             SWAN version:41.51A
//! %
//! %       Xp            Yp            Depth         Hsig     ...
//! %       [degr]        [degr]        [m]           [m]      ...
//! %
//!       -71.545       41.3650       10.0192       1.19505    ...
//! ```

use std::{error::Error, fmt};

#[derive(Clone, Debug)]
pub struct SwanTableFile {
    pub columns: Vec<String>,
    /// Bracketed unit strings, aligned with `columns` (empty if the file
    /// carried none).
    pub units: Vec<String>,
    /// One row per output time.
    pub rows: Vec<Vec<f64>>,
}

impl SwanTableFile {
    pub fn from_data(data: &str) -> Result<Self, SwanTableError> {
        let mut columns: Vec<String> = Vec::new();
        let mut units: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<f64>> = Vec::new();

        for (index, line) in data.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(comment) = trimmed.strip_prefix('%') {
                let tokens: Vec<&str> = comment.split_whitespace().collect();
                if tokens.is_empty() || comment.contains(':') {
                    continue; // bare % or the Run/Table/version banner
                }
                if tokens.iter().all(|t| t.starts_with('[')) {
                    units = tokens
                        .iter()
                        .map(|t| t.trim_matches(|c| c == '[' || c == ']').to_string())
                        .collect();
                } else if tokens
                    .iter()
                    .all(|t| t.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
                {
                    columns = tokens.iter().map(|t| t.to_string()).collect();
                }
                continue;
            }

            let row: Vec<f64> = trimmed
                .split_whitespace()
                .map(|token| {
                    token.parse().map_err(|error| {
                        SwanTableError::parse(line_number, format!("invalid value: {error}"))
                    })
                })
                .collect::<Result<_, _>>()?;
            if !columns.is_empty() && row.len() != columns.len() {
                return Err(SwanTableError::parse(
                    line_number,
                    format!("{} values for {} columns", row.len(), columns.len()),
                ));
            }
            rows.push(row);
        }

        if rows.is_empty() {
            return Err(SwanTableError::Empty);
        }
        Ok(SwanTableFile {
            columns,
            units,
            rows,
        })
    }

    /// Column index by (case-insensitive) name.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name))
    }

    /// Value at (row, column name).
    pub fn value(&self, row: usize, name: &str) -> Option<f64> {
        self.rows.get(row)?.get(self.column(name)?).copied()
    }
}

#[derive(Debug)]
pub enum SwanTableError {
    Parse { line: usize, message: String },
    Empty,
}

impl SwanTableError {
    fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for SwanTableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { line, message } => {
                write!(formatter, "SWAN table parse error on line {line}: {message}")
            }
            Self::Empty => write!(formatter, "no data rows found in SWAN table output"),
        }
    }
}

impl Error for SwanTableError {}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = "%\n\
%\n\
% Run:01    Table:SPT00             SWAN version:41.51A\n\
%\n\
%       Xp            Yp            Depth         Hsig          RTpeak        TPsmoo        Dir           Dspr     \n\
%       [degr]        [degr]        [m]           [m]           [sec]         [sec]         [degr]        [degr]   \n\
%\n\
      -71.545       41.3650       10.0192       1.19505        5.6422        5.4159       126.826       23.7477\n";

    #[test]
    fn parses_head_table_with_named_columns() {
        let table = SwanTableFile::from_data(EXAMPLE).unwrap();
        assert_eq!(
            table.columns,
            ["Xp", "Yp", "Depth", "Hsig", "RTpeak", "TPsmoo", "Dir", "Dspr"]
        );
        assert_eq!(table.units[3], "m");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.value(0, "hsig"), Some(1.19505));
        assert_eq!(table.value(0, "Dir"), Some(126.826));
        assert_eq!(table.value(0, "nope"), None);
    }
}
