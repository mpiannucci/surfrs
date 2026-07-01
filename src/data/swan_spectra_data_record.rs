use std::f64::consts::PI;
use std::fmt::Write as _;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::location::Location;
use crate::spectra::Spectra;
use crate::units::direction::DirectionConvention;

use super::parseable_data_record::DataRecordParsingError;

/// SWAN VaDens is per degree, Spectra energy is per radian.
const RAD_PER_DEG: f64 = PI / 180.0;

/// Standard SWAN "missing data" value.
const EXCEPTION_VALUE: f64 = -99.0;

/// Integer scaling range used when writing the FACTOR-scaled density matrix.
const FACTOR_RANGE: f64 = 1.0e6;

/// A single SWAN spectrum: one location, one (optional) time.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SwanSpectraDataRecord {
    pub date: Option<DateTime<Utc>>,
    pub location: Location,
    pub spectra: Spectra,
}

/// Shared grid metadata parsed from a SWAN spectral file header.
#[derive(Clone, Debug)]
pub struct SwanSpectraHeader {
    pub time_dependent: bool,
    pub locations: Vec<Location>,
    /// Frequency bins in Hz
    pub frequency: Vec<f64>,
    /// Direction bins in radians
    pub direction: Vec<f64>,
    pub dir_convention: DirectionConvention,
    pub exception: f64,
}

/// Parse a `yyyymmdd.hhmmss` SWAN timestamp.
fn parse_swan_date(token: &str) -> Result<DateTime<Utc>, DataRecordParsingError> {
    let (d, t) = token.split_once('.').ok_or_else(|| {
        DataRecordParsingError::ParseFailure(format!("Invalid SWAN date: {token}"))
    })?;
    if d.len() < 8 || t.len() < 6 {
        return Err(DataRecordParsingError::ParseFailure(format!(
            "Invalid SWAN date: {token}"
        )));
    }

    let year = d[0..4].parse().map_err(DataRecordParsingError::from)?;
    let month = d[4..6].parse().map_err(DataRecordParsingError::from)?;
    let day = d[6..8].parse().map_err(DataRecordParsingError::from)?;
    let hour = t[0..2].parse().map_err(DataRecordParsingError::from)?;
    let minute = t[2..4].parse().map_err(DataRecordParsingError::from)?;
    let second = t[4..6].parse().map_err(DataRecordParsingError::from)?;

    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .ok_or_else(|| DataRecordParsingError::ParseFailure(format!("Invalid SWAN date: {token}")))
}

/// Line cursor over meaningful content, skipping blanks and `$` comments.
struct Cursor<'a> {
    lines: Vec<&'a str>,
    idx: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a str) -> Self {
        let lines = data
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('$'))
            .collect();
        Cursor { lines, idx: 0 }
    }

    fn next(&mut self) -> Result<&'a str, DataRecordParsingError> {
        let line = self
            .lines
            .get(self.idx)
            .copied()
            .ok_or(DataRecordParsingError::EOF)?;
        self.idx += 1;
        Ok(line)
    }

    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.idx).copied()
    }
}

/// First whitespace token of a line.
fn head(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

fn parse_count(line: &str) -> Result<usize, DataRecordParsingError> {
    head(line).parse().map_err(DataRecordParsingError::from)
}

fn parse_f64(token: &str) -> Result<f64, DataRecordParsingError> {
    token.parse().map_err(DataRecordParsingError::from)
}

/// Read `n` values, one per line (first token of each).
fn read_values(cur: &mut Cursor, n: usize) -> Result<Vec<f64>, DataRecordParsingError> {
    let mut values = Vec::with_capacity(n);
    while values.len() < n {
        values.push(parse_f64(head(cur.next()?))?);
    }
    Ok(values)
}

impl SwanSpectraDataRecord {
    /// Parse a SWAN standard spectral (2D) file into its header and records.
    pub fn from_swan(
        data: &str,
    ) -> Result<(SwanSpectraHeader, Vec<SwanSpectraDataRecord>), DataRecordParsingError> {
        let mut cur = Cursor::new(data);

        let first = cur.next()?;
        if !first.contains("SWAN") {
            return Err(DataRecordParsingError::ParseFailure(
                "Missing SWAN header".into(),
            ));
        }

        let mut time_dependent = false;
        if head(cur.peek().unwrap_or("")) == "TIME" {
            cur.next()?;
            cur.next()?; // time coding option
            time_dependent = true;
        }

        // Locations (x = longitude, y = latitude)
        cur.next()?; // LOCATIONS / LONLAT
        let nloc = parse_count(cur.next()?)?;
        let mut locations = Vec::with_capacity(nloc);
        for _ in 0..nloc {
            let line = cur.next()?;
            let mut it = line.split_whitespace();
            let x = parse_f64(it.next().unwrap_or(""))?;
            let y = parse_f64(it.next().unwrap_or(""))?;
            locations.push(Location::new(y, x, String::new()));
        }

        // Frequencies (RFREQ relative / AFREQ absolute)
        cur.next()?;
        let nfreq = parse_count(cur.next()?)?;
        let frequency = read_values(&mut cur, nfreq)?;

        // Directions (NDIR nautical / CDIR cartesian)
        let dir_kw = cur.next()?;
        let dir_convention = if head(dir_kw).eq_ignore_ascii_case("CDIR") {
            DirectionConvention::Met
        } else {
            DirectionConvention::From
        };
        let ndir = parse_count(cur.next()?)?;
        let direction: Vec<f64> = read_values(&mut cur, ndir)?
            .iter()
            .map(|d| d.to_radians())
            .collect();

        // Quantities: only single quantity 2D VaDens is supported
        cur.next()?; // QUANT
        let nquant = parse_count(cur.next()?)?;
        if nquant != 1 {
            return Err(DataRecordParsingError::ParseFailure(format!(
                "Expected single quantity 2D spectra, found {nquant}"
            )));
        }
        cur.next()?; // VaDens
        cur.next()?; // unit
        let exception = parse_f64(head(cur.next()?))?;

        let header = SwanSpectraHeader {
            time_dependent,
            locations,
            frequency,
            direction,
            dir_convention,
            exception,
        };

        let mut records = Vec::new();
        loop {
            let date = if header.time_dependent {
                match cur.peek() {
                    Some(line) => {
                        let d = parse_swan_date(head(line))?;
                        cur.next()?;
                        Some(d)
                    }
                    None => break,
                }
            } else {
                None
            };

            for location in &header.locations {
                if cur.peek().is_none() {
                    break;
                }
                if let Some(spectra) = read_spectrum(&mut cur, &header)? {
                    records.push(SwanSpectraDataRecord {
                        date,
                        location: location.clone(),
                        spectra,
                    });
                }
            }

            if !header.time_dependent || cur.peek().is_none() {
                break;
            }
        }

        Ok((header, records))
    }

    /// Serialize a set of records to the SWAN standard spectral (2D) format.
    /// The frequency/direction grid of the first record is used for all.
    pub fn to_swan(records: &[SwanSpectraDataRecord]) -> Result<String, DataRecordParsingError> {
        let template = &records
            .first()
            .ok_or_else(|| DataRecordParsingError::ParseFailure("No records to write".into()))?
            .spectra;

        let frequency = &template.frequency;
        let direction_deg = template.direction_deg();
        let time_dependent = records.iter().any(|r| r.date.is_some());

        // Distinct locations and dates, preserving first-seen order
        let mut locations: Vec<Location> = Vec::new();
        for r in records {
            if !locations.contains(&r.location) {
                locations.push(r.location.clone());
            }
        }
        let mut dates: Vec<DateTime<Utc>> = Vec::new();
        for r in records.iter().filter_map(|r| r.date) {
            if !dates.contains(&r) {
                dates.push(r);
            }
        }

        let mut out = String::new();
        let _ = writeln!(out, "SWAN   1                                Swan standard spectral file, version");
        let _ = writeln!(out, "$   Data produced by surfrs");
        if time_dependent {
            let _ = writeln!(out, "TIME                                    time-dependent data");
            let _ = writeln!(out, "     1                                  time coding option");
        }
        let _ = writeln!(out, "LOCATIONS                               locations in x-y-space");
        let _ = writeln!(out, "{:>6}                                  number of locations", locations.len());
        for loc in &locations {
            let _ = writeln!(out, "{:>16.6}{:>16.6}", loc.longitude, loc.latitude);
        }
        let _ = writeln!(out, "RFREQ                                   relative frequencies in Hz");
        let _ = writeln!(out, "{:>6}                                  number of frequencies", frequency.len());
        for f in frequency {
            let _ = writeln!(out, "{:>12.6}", f);
        }
        let _ = writeln!(out, "NDIR                                    spectral nautical directions in degr");
        let _ = writeln!(out, "{:>6}                                  number of directions", direction_deg.len());
        for d in &direction_deg {
            let _ = writeln!(out, "{:>12.4}", d);
        }
        let _ = writeln!(out, "QUANT");
        let _ = writeln!(out, "     1                                  number of quantities in table");
        let _ = writeln!(out, "VaDens                                  variance densities in m2/Hz/degr");
        let _ = writeln!(out, "m2/Hz/degr                              unit");
        let _ = writeln!(out, "{:>16.4E}                          exception value", EXCEPTION_VALUE);

        if time_dependent {
            for date in &dates {
                let _ = writeln!(out, "{}", date.format("%Y%m%d.%H%M%S"));
                for loc in &locations {
                    match records.iter().find(|r| r.date == Some(*date) && &r.location == loc) {
                        Some(r) => write_spectrum(&mut out, &r.spectra),
                        None => {
                            let _ = writeln!(out, "NODATA");
                        }
                    }
                }
            }
        } else {
            for loc in &locations {
                match records.iter().find(|r| &r.location == loc) {
                    Some(r) => write_spectrum(&mut out, &r.spectra),
                    None => {
                        let _ = writeln!(out, "NODATA");
                    }
                }
            }
        }

        Ok(out)
    }
}

/// Read a single spectrum block (FACTOR / ZERO / NODATA) for one location.
fn read_spectrum(
    cur: &mut Cursor,
    header: &SwanSpectraHeader,
) -> Result<Option<Spectra>, DataRecordParsingError> {
    let nk = header.frequency.len();
    let nd = header.direction.len();

    let kw = head(cur.next()?).to_uppercase();
    match kw.as_str() {
        "NODATA" => Ok(None),
        "ZERO" => Ok(Some(Spectra::new(
            header.frequency.clone(),
            header.direction.clone(),
            vec![0.0; nk * nd],
            header.dir_convention.clone(),
        ))),
        "FACTOR" => {
            let factor = parse_f64(head(cur.next()?))?;
            let mut energy = vec![0.0; nk * nd];
            for ik in 0..nk {
                let row: Vec<f64> = cur
                    .next()?
                    .split_whitespace()
                    .map(parse_f64)
                    .collect::<Result<_, _>>()?;
                if row.len() < nd {
                    return Err(DataRecordParsingError::ParseFailure(
                        "SWAN density row shorter than direction count".into(),
                    ));
                }
                for ith in 0..nd {
                    // factor * value -> per degree; convert to per radian
                    energy[ik + ith * nk] = factor * row[ith] / RAD_PER_DEG;
                }
            }
            Ok(Some(Spectra::new(
                header.frequency.clone(),
                header.direction.clone(),
                energy,
                header.dir_convention.clone(),
            )))
        }
        other => Err(DataRecordParsingError::ParseFailure(format!(
            "Unexpected SWAN spectrum keyword: {other}"
        ))),
    }
}

/// Write a single spectrum block (FACTOR-scaled integers, or ZERO).
fn write_spectrum(out: &mut String, spectra: &Spectra) {
    let nk = spectra.nk();
    let nd = spectra.nth();

    // Per-degree densities and peak magnitude
    let mut max_deg = 0.0_f64;
    let mut deg = vec![0.0; nk * nd];
    for ik in 0..nk {
        for ith in 0..nd {
            let v = spectra.energy_at(ik, ith) * RAD_PER_DEG;
            deg[ik + ith * nk] = v;
            max_deg = max_deg.max(v.abs());
        }
    }

    if max_deg <= 0.0 {
        let _ = writeln!(out, "ZERO");
        return;
    }

    let factor = max_deg / FACTOR_RANGE;
    let _ = writeln!(out, "FACTOR");
    let _ = writeln!(out, "{:>18.8E}", factor);
    for ik in 0..nk {
        let mut line = String::new();
        for ith in 0..nd {
            let scaled = (deg[ik + ith * nk] / factor).round() as i64;
            let _ = write!(line, "{scaled:>8}");
        }
        let _ = writeln!(out, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SwanSpectraDataRecord {
        let frequency = vec![0.05, 0.06, 0.07, 0.08];
        let direction: Vec<f64> = (0..8).map(|i| (i as f64 * 45.0).to_radians()).collect();
        let mut energy = vec![0.0; frequency.len() * direction.len()];
        for ik in 0..frequency.len() {
            for ith in 0..direction.len() {
                energy[ik + ith * frequency.len()] = (ik + 1) as f64 * (ith + 1) as f64 * 0.01;
            }
        }
        SwanSpectraDataRecord {
            date: Some(Utc.with_ymd_and_hms(2024, 3, 4, 12, 0, 0).unwrap()),
            location: Location::new(41.0, -71.0, String::new()),
            spectra: Spectra::new(frequency, direction, energy, DirectionConvention::From),
        }
    }

    #[test]
    fn round_trip_swan_spectra() {
        let record = sample();
        let text = SwanSpectraDataRecord::to_swan(std::slice::from_ref(&record)).unwrap();

        let (header, records) = SwanSpectraDataRecord::from_swan(&text).unwrap();
        assert!(header.time_dependent);
        assert_eq!(header.frequency.len(), 4);
        assert_eq!(header.direction.len(), 8);
        assert_eq!(records.len(), 1);

        let parsed = &records[0];
        assert_eq!(parsed.date, record.date);
        assert_eq!(parsed.location, record.location);

        for (a, b) in record
            .spectra
            .energy
            .iter()
            .zip(parsed.spectra.energy.iter())
        {
            assert!((a - b).abs() <= a.abs() * 1e-4 + 1e-9, "{a} vs {b}");
        }
    }

    #[test]
    fn parse_stationary_zero_block() {
        let text = "SWAN   1\n\
            LOCATIONS locations in x-y-space\n\
                 1\n\
                -71.0 41.0\n\
            RFREQ relative frequencies in Hz\n\
                 2\n\
                0.05\n\
                0.06\n\
            NDIR nautical directions\n\
                 2\n\
                0.0\n\
                180.0\n\
            QUANT\n\
                 1\n\
            VaDens\n\
            m2/Hz/degr\n\
            -0.9900E+02\n\
            ZERO\n";

        let (header, records) = SwanSpectraDataRecord::from_swan(text).unwrap();
        assert!(!header.time_dependent);
        assert_eq!(records.len(), 1);
        assert!(records[0].date.is_none());
        assert!(records[0].spectra.energy.iter().all(|e| *e == 0.0));
    }
}
