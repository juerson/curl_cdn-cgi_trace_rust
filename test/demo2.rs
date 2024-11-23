use serde::Deserialize;
use serde_json::Result;
use std::fs;

#[derive(Debug, Deserialize)]
struct Airport {
    iata: String,
    cca2: String,
    region: String,
    city: String,
}

fn main() -> Result<()> {
    let locations = fs::read_to_string("locations.json").expect("Unable to read file");

    let airports: Vec<Airport> = serde_json::from_str(&locations)?;

    // 查找 iata = "HRE" 的
    let target_iata = "HRE";
    if let Some(airport) = airports.iter().find(|a| a.iata == target_iata) {
        println!("CCA2: {}", airport.cca2);
        println!("Region: {}", airport.region);
        println!("City: {}", airport.city);
    } else {
        println!("Airport with IATA code '{}' not found", target_iata);
    }

    Ok(())
}
