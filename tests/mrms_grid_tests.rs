// use std::{path::Path, fs::{File, self}, io::Read};

// use gribberish::{message::read_messages, data_message::DataMessage};
// use surfrs::tools::contour::compute_latlng_gridded_contours;

// #[test]
// fn contour_radar() {
//     let grib_path = format!("mock/MRMS_MergedReflectivityQCComposite_00.50_20230503-000037.grib2");

//     let grib_path = Path::new(&grib_path);
//     let mut grib_file = File::open(grib_path).expect("file not found");

//     let mut buf: Vec<u8> = Vec::new();
//     grib_file
//         .read_to_end(&mut buf)
//         .expect("Failed to read data from the grib file");

//     let messages = read_messages(&buf).collect::<Vec<_>>();
//     let message = messages.first().unwrap();
//     let data_message = DataMessage::try_from(message).unwrap();

//     let data = data_message.flattened_data();
//     let (lat_count, lng_count) = data_message.metadata.grid_shape;
//     let (lng_start, lat_start, lng_end, lat_end) = data_message.metadata.bbox;

//     let collection = compute_latlng_gridded_contours(
//         data,
//         lng_count, 
//         lat_count, 
//         lng_start, 
//         lat_start, 
//         lng_end, 
//         lat_end, 
//         Some(0.0), 
//         Some(80.0), 
//         Some(16), 
//         Some(|_: &usize, value: &f64| {
//             format!("{value:.0}")
//         })
//     ).unwrap();

//     let contour_data = serde_json::to_string(&collection).unwrap();
//     _ = fs::write("mrms.json", contour_data);
// }