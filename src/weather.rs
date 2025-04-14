use crate::location::Location;

const API_ROOT_URL: &str = "https://api.weather.gov/";

pub fn create_points_url(location: &Location) -> String {
    format!(
        "{API_ROOT_URL}points/{lat},{lon}",
        lat = location.latitude,
        lon = location.longitude
    )
}

pub fn create_gridpoints_url(office: &str, grid_x: &usize, grid_y: &usize) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}")
}

pub fn create_forecast_url(office: &str, grid_x: &usize, grid_y: &usize) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}/forecast")
}

pub fn create_hourly_forecast_url(office: &str, grid_x: &usize, grid_y: &usize) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}/forecast/hourly")
}
