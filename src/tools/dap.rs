use std::{fmt::Display, str::FromStr};

use gribberish::unwrap_or_return;

#[derive(Debug, Clone)]
pub struct DapConstraint {
    pub var: String,
    pub ranges: Vec<(usize, usize, usize)>,
}

impl DapConstraint {
    pub fn new(var: String, ranges: Vec<(usize, usize, usize)>) -> Self {
        Self { var, ranges }
    }
}

impl FromStr for DapConstraint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('[');
        let var = unwrap_or_return!(parts.next(), "No variable found for dap constraint".to_string()).to_string();
        let ranges = parts
            .map(|range| {
                let mut parts = range.split(':');
                let start = parts.next().unwrap().parse::<usize>().unwrap();
                let step = parts.next().unwrap().parse::<usize>().unwrap();
                let end = parts.next().unwrap().replace("]", "").parse::<usize>().unwrap();
                (start, step, end)
            })
            .collect::<Vec<(usize, usize, usize)>>();
        Ok(Self { var, ranges })
    }
}

impl Display for DapConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let range_str = self
            .ranges
            .iter()
            .map(|(start, step, end)| format!("[{}:{}:{}]", start, step, end))
            .collect::<Vec<String>>()
            .join("");

        let constraint_str = format!("{var}{range_str}", var = self.var);
        f.write_str(&constraint_str)
    }
}

pub fn format_dap_constraints(constraints: &[DapConstraint]) -> String {
    constraints
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

pub fn format_dods_url(url_root: &str, constraints: &[DapConstraint]) -> String {
    let constraints = format_dap_constraints(constraints);
    format!("{url_root}.dods?{constraints}")
}

#[cfg(test)]
mod tests {
    use crate::tools::dap::DapConstraint;

    #[test]
    fn test_read_dap_constraint() {
        let constraint = "wind[0:1:2][0:2:6]".parse::<DapConstraint>().unwrap();
        assert_eq!(constraint.var, "wind");
        assert_eq!(constraint.ranges, vec![(0, 1, 2), (0, 2, 6)]);
    }

    #[test]
    fn test_format_dap_constraint() {
        let constraint = DapConstraint::new("wind".to_string(), vec![(0, 1, 2), (0, 2, 6)]);
        assert_eq!(constraint.to_string(), "wind[0:1:2][0:2:6]");
    }
}