/**
 * This determines an imprecise geolocation based on IP address, via ip-api.com.
 * It is used to implement geo-fencing to protect from user session hijacking.
 */

use std::f64::consts::PI;
use std::io;
use std::error::Error;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct IpApiResponse {
    pub query: String,
    pub lat: f64,
    pub lon: f64,
    pub timezone: String,
}

#[allow(unused)]
#[derive(Debug, Default)]
pub struct Geolocation {
    pub ip: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

pub async fn find(ip: &str) -> Result<Geolocation, Box<dyn Error>>{
    if ip == "" {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "IP Address Not Provided.")))
    }

    let uri = format!("http://ip-api.com/json/{}", &ip);
    let client = Client::new();

    let response = client
        .get(uri)
        .send()
        .await?
        .json::<IpApiResponse>()
        .await?;

    let result = Geolocation {
        ip: response.query,
        latitude: response.lat,
        longitude: response.lon,
        timezone: response.timezone,
    };

    Ok(result)
}

fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let earth_radius_km = 6371.0;

    let dlat = degrees_to_radians(lat2 - lat1);
    let dlon = degrees_to_radians(lon2 - lon1);

    let lat1 = degrees_to_radians(lat1);
    let lat2 = degrees_to_radians(lat2);

    let a = (dlat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    earth_radius_km * c
}

pub fn is_within_geo_fence(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> bool {
    return haversine(lat1, lon1, lat2, lon2) <= 100.0; // Within 100km
}
