use std::{path::Path, fs::File, io::Read};

use gribberish::message::{read_messages};
use surfrs::{model::GFSWaveModel, location::Location, data::gfs_wave_grib_point_data_record::GFSWaveGribPointDataRecord};

#[test]
fn extract_wave_data_record() {
    let model = GFSWaveModel::atlantic();
    let grib_path = format!("mock/gfswave.t12z.atlocn.0p16.f087.grib2");
    let location = Location::new(41.35, -71.4, "Block Island Sound".into());

    let grib_path = Path::new(&grib_path);
    let mut grib_file = File::open(grib_path).expect("file not found");
    
    let mut buf: Vec<u8> = Vec::new();
    grib_file.read_to_end(&mut buf).expect("Failed to read data from the grib file");

    let messages = read_messages(&buf).collect::<Vec<_>>();
    let wave_data = GFSWaveGribPointDataRecord::from_messages(&model, &messages, &location);

    println!("{:?}", wave_data);
}