extern crate surfrs;

use std::fs;
// use sur::data::buoy_data::BuoyDataRecord;

// #[test]
// fn read_meteorological_data() {
//     let raw_data = fs::read_to_string("mock/44017.met.txt").unwrap();
//     let read_data = BuoyDataRecord::parse_from_meteorological_data(raw_data.as_str());
//     assert_ne!(read_data.is_none(), true);
//     if let Some(met_data) = read_data {
//         match met_data {
//             BuoyDataRecord::Meteorological(d) => {
//                 assert_eq!(d.tide.value.is_none(), true);

//                 // TODO: More tests
//             },
//             _ => {}
//         };
//     }
// }