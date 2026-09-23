use gribberish::index::IndexEntry;
use serde::{Deserialize, Serialize};

/// A span of bytes in a remote GRIB file, for HTTP range requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub offset: u64,
    pub length: u64,
}

impl ByteRange {
    /// The range an index entry points at, or None when its length is unknown
    /// (the last entry of a NOAA index without the file size).
    pub fn from_index_entry(entry: &IndexEntry) -> Option<Self> {
        entry.length.map(|length| ByteRange {
            offset: entry.offset,
            length,
        })
    }

    /// Exclusive end offset.
    pub fn end(&self) -> u64 {
        self.offset + self.length
    }

    /// The value for an HTTP `Range` header, e.g. `bytes=0-99`.
    pub fn http_range_header(&self) -> String {
        format!("bytes={}-{}", self.offset, self.end().saturating_sub(1))
    }
}

/// Sort ranges and merge any that overlap or are separated by at most `max_gap`
/// bytes, so fewer requests are needed. Bytes in the gaps are fetched too.
pub fn coalesce_ranges(mut ranges: Vec<ByteRange>, max_gap: u64) -> Vec<ByteRange> {
    ranges.sort_by_key(|r| r.offset);

    let mut merged: Vec<ByteRange> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.offset <= last.end() + max_gap => {
                last.length = last.end().max(range.end()) - last.offset;
            }
            _ => merged.push(range),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(offset: u64, length: u64) -> ByteRange {
        ByteRange { offset, length }
    }

    #[test]
    fn test_http_range_header() {
        assert_eq!(range(0, 100).http_range_header(), "bytes=0-99");
        assert_eq!(
            range(806097, 786346).http_range_header(),
            "bytes=806097-1592442"
        );
    }

    #[test]
    fn test_coalesce_adjacent_and_gapped() {
        let merged = coalesce_ranges(vec![range(200, 50), range(0, 100), range(100, 50)], 0);
        assert_eq!(merged, vec![range(0, 150), range(200, 50)]);

        let merged = coalesce_ranges(vec![range(200, 50), range(0, 100), range(100, 50)], 50);
        assert_eq!(merged, vec![range(0, 250)]);
    }

    #[test]
    fn test_coalesce_overlapping() {
        let merged = coalesce_ranges(vec![range(0, 100), range(50, 20), range(90, 30)], 0);
        assert_eq!(merged, vec![range(0, 120)]);
    }
}
