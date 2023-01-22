use contour::ContourBuilder;
use geojson::{Feature, Value};
use itertools::Either;
use serde::{Deserialize, Serialize};

use crate::location::normalize_longitude;

use super::{linspace::linspace, vector::min_max_fill};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContourError {
    ContourFailure,
}

pub fn compute_contours<C: Fn(&Vec<f64>) -> Vec<f64>, F: Fn(&usize, &f64) -> String>(
    data: &[f64],
    width: usize,
    height: usize,
    thresholds: &[f64],
    coordinate_transform: Option<C>,
    label_format: Option<F>
) -> Result<Vec<Feature>, ContourError> {
    let contour_builder = ContourBuilder::new(width as u32, height as u32, true);

    let features = contour_builder
        .contours(&data, &thresholds)
        .map_err(|_| ContourError::ContourFailure)?
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut f = c.to_geojson();

            if coordinate_transform.is_none() {
                return f;
            }

            let coordinate_transform = coordinate_transform.as_ref().unwrap();

            let Some(g) = &f.geometry else {
                return f;
            };

            let geo_value: Value = g.value.clone();

            let Value::MultiPolygon(coords) = geo_value else {
                return f;
            };

            let new_coordinates: Vec<Vec<Vec<Vec<f64>>>> = coords
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|c| {
                            c.iter()
                                .map(|point| coordinate_transform(point))
                                .collect()
                        })
                        .collect()
                })
                .collect();
            let new_polygon = Value::MultiPolygon(new_coordinates);
            f.geometry = Some(new_polygon.into());

            if let Some(formatter) = label_format.as_ref() {
                let label = formatter(&i, &c.threshold());
                f.set_property("label", label);
            }

            f
        })
        .collect::<Vec<_>>();

    Ok(features)
}

pub fn compute_latlng_gridded_contours<F: Fn(&usize, &f64) -> String>(
    data: Vec<f64>,
    lng_count: usize, 
    lat_count: usize,
    lng_start: f64, 
    lng_end: f64, 
    lat_start: f64, 
    lat_end: f64, 
    threshold_min: Option<f64>,
    threshold_max: Option<f64>,
    threshold_count: Option<usize>,
    label_format: Option<F>
) -> Result<Vec<Feature>, ContourError> {
    let lng_step = (lng_end - lng_start) / lng_count as f64;
    let diff = normalize_longitude(lng_end) - normalize_longitude(lng_start);

    let mut data_count = lat_count * lng_count;
    let mut lng_end = lng_end;
    let mut lng_count = lng_count;

    let mut data = if diff.abs() - lng_step.abs() < 0.001 {
        let data = data
            .iter()
            .enumerate()
            .flat_map(|(i, v)| {
                let lng_index = i % lng_count;
                if lng_index == lng_count - 1 {
                    let lat_index = i / lng_count;
                    Either::Left([*v, *(&data[lat_index * lng_count])].into_iter())
                } else {
                    Either::Right(std::iter::once(*v))
                }
            })
            .collect();
        lng_end = (lng_end + lng_step).ceil();
        lng_count += 1;
        data_count = lat_count * lng_count;
        data
    } else {
        data
    };

    let (min, max) = min_max_fill(&mut data, -99999.0);
    let thresholds = linspace(
        threshold_min.unwrap_or(min),
        threshold_max.unwrap_or(max),
        threshold_count.unwrap_or(20),
    )
    .collect::<Vec<_>>();

    compute_contours(
        &data[0..data_count],
        lng_count,
        lat_count,
        &thresholds,
        Some(|point: &Vec<f64>| {
            let x = lng_start + (lng_end - lng_start) * (point[0] / (lng_count as f64));
            let y = lat_start + (lat_end - lat_start) * (point[1] / (lat_count as f64));
            vec![x, y]
        }),
        label_format,
    )
}