use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use geojson::FeatureCollection;
use gribberish::message::read_messages;
use surfrs::{
    data::gfs_wave_grib_point_data_record::GFSWaveGribPointDataRecord, location::Location,
    model::GFSWaveModel, model::NOAAModel,
};

#[test]
fn extract_atlantic_wave_data_record() {
    let model = GFSWaveModel::atlantic();
    let grib_path = format!("mock/gfswave.t12z.atlocn.0p16.f087.grib2");
    let location = Location::new(41.35, -71.4, "Block Island Sound".into());

    let grib_path = Path::new(&grib_path);
    let mut grib_file = File::open(grib_path).expect("file not found");

    let mut buf: Vec<u8> = Vec::new();
    grib_file
        .read_to_end(&mut buf)
        .expect("Failed to read data from the grib file");

    let messages = read_messages(&buf).collect::<Vec<_>>();
    let wave_data = GFSWaveGribPointDataRecord::from_messages(&model, &messages, &location);

    println!("{}", wave_data.date);
    println!("{:?}", wave_data.swell_components[0].to_string());
    println!("{:?}", wave_data.swell_components[1].to_string());
    println!("{:?}", wave_data.swell_components[2].to_string());
    println!("{:?}", wave_data.swell_components[3].to_string());

    let wave_message = messages
        .iter()
        .find(|m| m.variable_abbrev().unwrap_or("".into()) == "HTSGW")
        .unwrap();
    let wave_features = model
        .contour_data(wave_message, Some(0.0), Some(12.0), Some(24), Some(surfrs::units::UnitSystem::English))
        .unwrap();
    let collection = FeatureCollection {
        bbox: None,
        features: wave_features,
        foreign_members: None,
    };

    let contour_data = serde_json::to_string(&collection).unwrap();
    _ = fs::write("wvsgw.json", contour_data);
}

#[test]
fn extract_global_wave_data_record() {
    let model = GFSWaveModel::atlantic();
    let grib_path = format!("mock/gfswave.t00z.global.0p25.f031.grib2");

    let grib_path = Path::new(&grib_path);
    let mut grib_file = File::open(grib_path).expect("file not found");

    let mut buf: Vec<u8> = Vec::new();
    grib_file
        .read_to_end(&mut buf)
        .expect("Failed to read data from the grib file");

    let messages = read_messages(&buf).collect::<Vec<_>>();

    let wave_message = messages
        .iter()
        .find(|m| m.variable_abbrev().unwrap_or("".into()) == "HTSGW")
        .unwrap();
    let wave_features = model
        .contour_data(wave_message, Some(0.0), Some(12.0), Some(24), None)
        .unwrap();
    let collection = FeatureCollection {
        bbox: None,
        features: wave_features,
        foreign_members: None,
    };

    let contour_data = serde_json::to_string(&collection).unwrap();
    _ = fs::write("global_wvsgw.json", contour_data);
}
