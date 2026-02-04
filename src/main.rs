use clap::{Parser, Subcommand};
use geotemporal_uuid::GeoTemporalUuid;
use chrono::{Utc, TimeZone, DateTime};
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "geotemporal_uuid")]
#[command(about = "GeoTemporal UUID Generator & Decoder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new GeoTemporal UUID
    Generate {
        /// Latitude (-90 to 90)
        #[arg(long)]
        lat: f64,
        
        /// Longitude (-180 to 180)
        #[arg(long)]
        lon: f64,
        
        /// Optional Timestamp (ms or ISO-8601). Defaults to now.
        #[arg(long)]
        time: Option<String>,
    },
    /// Decode an existing UUID
    Decode {
        /// The UUID string
        uuid: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { lat, lon, time } => {
            let dt = if let Some(t_str) = time {
                if let Ok(ms) = t_str.parse::<i64>() {
                    Utc.timestamp_millis_opt(ms).unwrap()
                } else {
                     DateTime::parse_from_rfc3339(&t_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .expect("Invalid time format. Use ms integer or ISO-8601")
                }
            } else {
                Utc::now()
            };

            match GeoTemporalUuid::new(lat, lon, Some(dt)) {
                Ok(uuid) => println!("{}", uuid),
                Err(e) => eprintln!("Error: {}", e),
            }
        },
        Commands::Decode { uuid } => {
             match GeoTemporalUuid::from_str(&uuid) {
                Ok(u) => {
                    let (lat, lon, time) = u.decode();
                    println!("UUID: {}", u);
                    println!("Time: {} ({})", time, time.timestamp_millis());
                    println!("Lat:  {:.6}", lat);
                    println!("Lon:  {:.6}", lon);
                },
                Err(e) => eprintln!("Error decoding: {}", e),
             }
        }
    }
}
