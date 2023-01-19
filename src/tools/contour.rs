use contour::ContourBuilder;
use geojson::{Feature, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContourError {
    ContourFailure,
}

pub fn compute_contours<F: Fn(&Vec<f64>) -> Vec<f64>>(
    data: &[f64],
    width: usize,
    height: usize,
    thresholds: &[f64],
    coordinate_transform: Option<F>,
) -> Result<Vec<Feature>, ContourError> {
    let contour_builder = ContourBuilder::new(width as u32, height as u32, true);

    let features = contour_builder
        .contours(&data, &thresholds)
        .map_err(|_| ContourError::ContourFailure)?
        .iter()
        .map(|c| {
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

            f
        })
        .collect::<Vec<_>>();

    Ok(features)
}