

const API_ROOT_URL: &str = "https://api.weather.gov/";

pub fn create_points_url(lat: f64, lon: f64) -> String {
    format!("{API_ROOT_URL}points/{lat},{lon}")
}

pub fn create_gridpoints_url(office: &str, grid_x: i32, grid_y: i32) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}")
}

pub fn create_forecast_url(office: &str, grid_x: i32, grid_y: i32) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}/forecast")
}

pub fn create_hourly_forecast_url(office: &str, grid_x: i32, grid_y: i32) -> String {
    format!("{API_ROOT_URL}gridpoints/{office}/{grid_x},{grid_y}/forecast/hourly")
}