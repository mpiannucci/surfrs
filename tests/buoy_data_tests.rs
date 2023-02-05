extern crate surfrs;

use std::f64::consts::PI;
use std::fs;
use std::time::Instant;
use surfrs::data::forecast_cbulletin_wave_data_record::{
    ForecastCBulletinWaveRecord, ForecastCBulletinWaveRecordCollection,
};
use surfrs::data::forecast_spectral_wave_data_record::ForecastSpectralWaveDataRecordCollection;
use surfrs::data::latest_obs_data_record::LatestObsDataRecordCollection;
use surfrs::data::meteorological_data_record::{MeteorologicalDataRecordCollection, StdmetDataRecordCollection};
use surfrs::data::spectral_wave_data_record::{
    DirectionalSpectralWaveDataRecord, SpectralWaveDataRecordCollection,
};
use surfrs::data::swden_wave_data_record::SwdenWaveDataRecordCollection;
use surfrs::data::wave_data_record::WaveDataRecordCollection;
use surfrs::swell::{Swell, SwellProvider};
use surfrs::tools::vector::bin;
use surfrs::units::{UnitConvertible, UnitSystem};

fn read_mock_data(name: &str) -> String {
    fs::read_to_string(format!("mock/{}", name)).unwrap()
}

#[test]
fn read_latest_obs_data() {
    let raw_data = read_mock_data("latest_obs.txt");
    let mut data_collection = LatestObsDataRecordCollection::from_data(raw_data.as_str());
    let mut data_records = data_collection.records();

    // assert_eq!(data_records.count(), 865);

    let nantucket = data_records.find(|s| s.station_id == "44097");
    assert!(nantucket.is_some());

    let nantucket = nantucket.unwrap();
    assert!(nantucket.wave_height.value.is_some());
}

#[test]
fn read_meteorological_data() {
    let raw_data = read_mock_data("44017.met.txt");
    let mut data_collection = MeteorologicalDataRecordCollection::from_data(raw_data.as_str());
    let data_records = data_collection.records();

    assert_eq!(data_records.count(), 1099);
}

#[test]
fn read_wave_data() {
    let raw_data = read_mock_data("44097.spec");
    let mut data_collection = WaveDataRecordCollection::from_data(raw_data.as_str());
    let first_record = data_collection.records().next();

    assert!(first_record.is_some())
}

#[test]
fn read_wave_energy_data() {
    let raw_data = read_mock_data("44008.data_spec");
    let mut data_collection = SpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let records = data_collection.records();

    assert!(records.count() == 1084);
}

#[test]
fn read_wave_direction_data() {
    let raw_data = read_mock_data("44097.swdir");
    let mut data_collection = SpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let records = data_collection.records();

    assert!(records.count() == 1099);
}

#[test]
fn read_wave_spectra_data() {
    let raw_energy_data = read_mock_data("44097.data_spec");
    let raw_mean_wave_direction_data = read_mock_data("44097.swdir");
    let raw_primary_wave_direction_data = read_mock_data("44097.swdir2");
    let raw_first_polar_coefficient_data = read_mock_data("44097.swr1");
    let raw_second_polar_coefficient_data = read_mock_data("44097.swr2");

    let mut energy_data_collection =
        SpectralWaveDataRecordCollection::from_data(raw_energy_data.as_str());
    let mut mean_wave_direction_data_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_mean_wave_direction_data.as_str());
    let mut primary_wave_direction_data_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_primary_wave_direction_data.as_str());
    let mut first_polar_coefficient_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_first_polar_coefficient_data.as_str());
    let mut second_polar_coefficient_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_second_polar_coefficient_data.as_str());

    let dir_count = 36usize;
    let dir_step = (2.0 * PI) / dir_count as f64;
    let directions = (0..dir_count)
        .map(|i| dir_step * (i as f64))
        .collect::<Vec<f64>>();

    let records = itertools::izip!(
        energy_data_collection.records(),
        mean_wave_direction_data_collection.records(),
        primary_wave_direction_data_collection.records(),
        first_polar_coefficient_collection.records(),
        second_polar_coefficient_collection.records(),
    )
    .map(|(e, mwd, pwd, r1, r2)| {
        DirectionalSpectralWaveDataRecord::from_data_records(&directions, e, mwd, pwd, r1, r2)
    });

    let record = records.skip(6).next().unwrap();

    // println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    // println!("BUOY -- {}", record.date);
    // println!("{:?}", record.spectra.frequency);
    // println!("{:?}", record.spectra.oned());

    let swell_data = record.swell_data();
    assert!(swell_data.is_ok());
    // assert_eq!(out, control);

    let mut swell_data = record.swell_data().unwrap();

    let false_components = swell_data.probable_false_components();

    let _swell_components = swell_data
        .components
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    // let control = "0.7 m @ 4.5 s 168° sse, 0.6 m @ 12.5 s 120° ese, 0.6 m @ 10.5 s 112° ese, 0.5 m @ 3.8 s 160° sse";
    // let out = swell_components.join(", ");

    swell_data
        .components
        .iter_mut()
        .enumerate()
        .for_each(|(i, component)| {
            let is_false_positive = false_components.contains(&i);
            component.to_units(&UnitSystem::English);
            // println!(
            //     "BUOY -- {} {} {}",
            //     &component,
            //     component.energy.clone().unwrap(),
            //     !is_false_positive
            // );
        });

    println!("buoy dirs: {:?}", &record.spectra.direction_deg());
    let cart_e = record.spectra.project_cartesian(50, Some(25.0), None);
    let (min_e, max_e) = record.spectra.energy_range();
    let binned_cart_e = bin(&cart_e, &min_e, &max_e, &255);
    // println!("buoy cartesian: {:?}", binned_cart_e);

    // println!("buoy mwd:: {:?}", &record.spectra.mean_wave_direction_f());

    // let _ = fs::write(
    //     "contours2.json",
    //     &record.spectra.contoured().unwrap().to_string(),
    // );
}

#[test]
fn read_cbulletin_forecast_station_data() {
    let raw_data = read_mock_data("gfswave.44097.cbull");
    let mut data_collection = ForecastCBulletinWaveRecordCollection::from_data(raw_data.as_str());
    let bulletin_records_iter = data_collection.records();
    assert!(bulletin_records_iter.is_ok());

    let bulletin_records: Vec<ForecastCBulletinWaveRecord> =
        bulletin_records_iter.unwrap().1.collect();
    assert!(bulletin_records[0].swell_data().is_ok());
    for (_, record) in bulletin_records.iter().enumerate() {
        assert!(record.swell_data().is_ok());
    }
}

#[test]
fn read_spectral_forecast_station_data() {
    let raw_data = read_mock_data("gfswave.44097.spec");
    let mut data_collection =
        ForecastSpectralWaveDataRecordCollection::from_data(raw_data.as_str());
    let spectral_records_iter = data_collection.records();
    assert!(spectral_records_iter.is_ok());

    let spectral_records = spectral_records_iter.unwrap().1;
    let record = spectral_records.skip(6).next().unwrap();
    // println!("++++++++");
    // println!("FORECAST -- {}", record.date);
    // println!("{:?}", record.spectra.frequency);
    // println!("{:?}", record.spectra.oned());
    // println!("forecast watershed: {:?}", watershed(&record.spectra.energy, record.spectra.frequency.len(), record.spectra.direction.len(), 100).unwrap().0);

    // let watershed = watershed(&record.energy, record.frequency.len(), record.direction.len(), 100);
    // println!("{:?}", watershed.unwrap().0);

    let swell_data = record.swell_data().unwrap();

    let _swell_components = swell_data
        .components
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    // let control = "0.7 m @ 4.5 s 168° sse, 0.6 m @ 12.5 s 120° ese, 0.6 m @ 10.5 s 112° ese, 0.5 m @ 3.8 s 160° sse";
    // let out = swell_components.join(", ");

    for mut component in swell_data.components {
        component.to_units(&UnitSystem::English);
        // println!(
        //     "FORECAST -- {} {}",
        //     component.clone(),
        //     component.energy.unwrap()
        // );
    }

    // println!("forecast dirs: {:?}", record.spectra.direction_deg());
    // println!(
    //     "forecast cartesian: {:?}",
    //     record.spectra.project_cartesian(50, Some(25.0), None)
    // );

    // println!(
    //     "forecast mwd:: {:?}",
    //     &record.spectra.mean_wave_direction_f()
    // );

    // let _ = fs::write(
    //     "contours.json",
    //     &record.spectra.contoured().unwrap().to_string(),
    // );
}

#[test]
fn read_waimea_spectra_data() {
    let raw_energy_data = read_mock_data("waimea_overflow/51201.data_spec");
    let raw_mean_wave_direction_data = read_mock_data("waimea_overflow/51201.swdir");
    let raw_primary_wave_direction_data = read_mock_data("waimea_overflow/51201.swdir2");
    let raw_first_polar_coefficient_data = read_mock_data("waimea_overflow/51201.swr1");
    let raw_second_polar_coefficient_data = read_mock_data("waimea_overflow/51201.swr2");

    let mut energy_data_collection =
        SpectralWaveDataRecordCollection::from_data(raw_energy_data.as_str());
    let mut mean_wave_direction_data_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_mean_wave_direction_data.as_str());
    let mut primary_wave_direction_data_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_primary_wave_direction_data.as_str());
    let mut first_polar_coefficient_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_first_polar_coefficient_data.as_str());
    let mut second_polar_coefficient_collection =
        SpectralWaveDataRecordCollection::from_data(&raw_second_polar_coefficient_data.as_str());

    let dir_count = 36usize;
    let dir_step = (2.0 * PI) / dir_count as f64;
    let directions = (0..dir_count)
        .map(|i| dir_step * (i as f64))
        .collect::<Vec<f64>>();

    let records = itertools::izip!(
        energy_data_collection.records(),
        mean_wave_direction_data_collection.records(),
        primary_wave_direction_data_collection.records(),
        first_polar_coefficient_collection.records(),
        second_polar_coefficient_collection.records(),
    )
    .map(|(e, mwd, pwd, r1, r2)| {
        DirectionalSpectralWaveDataRecord::from_data_records(&directions, e, mwd, pwd, r1, r2)
    });

    let record = records.skip(3).next().unwrap();
    let swell_data = record.swell_data().unwrap();
}

#[test]
fn read_dap_swden_data() {
    let raw_data = fs::read("mock/44097w9999.swden.nc.dods").unwrap();

    let record_collection = SwdenWaveDataRecordCollection::from_data(&raw_data);

    let dir_count = 36usize;
    let dir_step = (2.0 * PI) / dir_count as f64;
    let direction = (0..dir_count)
        .map(|i| dir_step * (i as f64))
        .collect::<Vec<f64>>();

    let swells = record_collection
        .records()
        .map(|s| {
            DirectionalSpectralWaveDataRecord::new(
                &s.date,
                &direction,
                &s.frequency,
                &s.energy_spectra,
                &s.mean_wave_direction,
                &s.primary_wave_direction,
                &s.first_polar_coefficient,
                &s.second_polar_coefficient,
            )
        })
        .map(|d| d.swell_data().unwrap().summary)
        .collect::<Vec<Swell>>();

    assert_eq!(swells.len(), 11);
}

#[test]
fn read_dap_stdmet_data() {
    let raw_data = fs::read("mock/44008h9999.stdmet.nc.dods").unwrap();

    let record_collection = StdmetDataRecordCollection::from_data(&raw_data);

    let data = record_collection
        .records()
        .collect::<Vec<_>>();

    assert_eq!(data.len(), 11);
}
