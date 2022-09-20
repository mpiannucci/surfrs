use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Serialize, Deserialize};

use crate::location::Location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolarEvents {
    pub sunrise: DateTime<Utc>, 
    pub solarnoon: DateTime<Utc>, 
    pub sunset: DateTime<Utc>,
}

pub fn calculate_solar_events(location: Location, date: DateTime<Utc>) {
    let longitude = location.absolute_longitude(); 
    let latitude = location.absolute_latitude();

    // datetime days are numbered in the Gregorian calendar  
    // while the calculations from NOAA are distibuted as  
    // OpenOffice spreadsheets with days numbered from  
    // 1/1/1900. The difference are those numbers taken for  
    let day = (date - DateTime::<Utc>::from_utc(chrono::NaiveDate::parse_from_str("1/1/1900", "%d/%m/%Y")
    .unwrap()
    .and_hms(0, 0, 0), Utc)).num_days();
    println!("date: {day}");

    let time = (date.hour() as f64 + (date.minute() as f64/60.0) + (date.second() as f64/3600.0))/24.0;  

    let j_day = day as f64+2415018.5+time; // Julian day  
    let j_century = (j_day-2451545.0)/36525.0;  // Julian century  

    let manom = 357.52911+j_century*(35999.05029-0.0001537*j_century);
    let mlong = 280.46646+j_century*(36000.76983+j_century*0.0003032)%360.0;  
    let eccent = 0.016708634-j_century*(0.000042037+0.0001537*j_century);
    let mobliq = 23.0+(26.0+((21.448-j_century*(46.815+j_century*(0.00059-j_century*0.001813))))/60.0)/60.0;
    let obliq = mobliq+0.00256*(125.04-1934.136*j_century).to_radians().cos();
    let vary     = (obliq/2.0).to_radians().tan()*(obliq/2.0).to_radians().tan();
    let seqcent  = manom.to_radians().sin()*(1.914602-j_century*(0.004817+0.000014*j_century))+(2.0*manom).to_radians().sin()*(0.019993-0.000101*j_century)+(3.0*manom).to_radians().sin()*0.000289;
    
    let struelong= mlong+seqcent;
    let sapplong = struelong-0.00569-0.00478*(125.04-1934.136*j_century).to_radians().sin();
    let declination = (obliq.to_radians().sin() * sapplong.to_radians().sin()).asin().to_degrees();

    let eqtime = 4.0 * (vary * (2.0 * mlong.to_radians()).sin() - (2.0 * eccent * manom.to_radians().sin()) + (4.0 * eccent * vary * manom.to_radians().sin() * (2.0 * mlong.to_radians()).cos()) - 0.5 * vary * vary * (4.0 * mlong.to_radians().sin()) - 1.25*eccent * eccent * (2.0 * manom.to_radians()).sin()).to_degrees();
   
    let hour_angle = (90.833f64.to_radians().cos()/(latitude.to_radians().cos() * declination.to_radians().cos()) - latitude.to_radians().tan() * declination.to_radians().tan()).acos().to_degrees();
    
    let solarnoon_t=(720.0-4.0*longitude-eqtime)/1440.0;
    let sunrise_t = solarnoon_t-hour_angle*4.0/1440.0;
    let sunset_t = solarnoon_t+hour_angle*4.0/1440.0;
    println!("Solarnoon {solarnoon_t}");
    println!("Sunrise {sunrise_t}");
    println!("Sunset {sunset_t}");
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDateTime, NaiveDate, NaiveTime, Utc, Datelike};

    use crate::location::Location;

    use super::calculate_solar_events;

    #[test]
    fn test_solar_events() {
        let location = Location::new(41.6, -71.5, "Narragansett Pier".into());

        println!("{}", (Utc::now() - DateTime::<Utc>::from_utc(chrono::NaiveDate::parse_from_str("1/1/1900", "%d/%m/%Y")
        .unwrap()
        .and_hms(0, 0, 0), Utc)).num_days());

        let n_date = NaiveDate::from_ymd(2022, 07, 15);
        let n_time = NaiveTime::from_hms(0, 0, 0);
        let n_datetime = NaiveDateTime::new(n_date, n_time);
        let date: DateTime<Utc> = DateTime::<Utc>::from_utc(n_datetime, Utc);

        let result = calculate_solar_events(location, date);
    }
}
