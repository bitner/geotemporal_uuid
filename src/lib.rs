use chrono::{DateTime, TimeZone, Utc};
use rand::Rng;
use std::fmt;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GeoTemporalUuid([u8; 16]);

impl GeoTemporalUuid {
    /// Latitude (24 bits)
    const LAT_BITS: u64 = 24;
    /// Longitude (25 bits)
    const LON_BITS: u64 = 25;
    /// Random (41 bits) - Increased for collision resistance
    const RAND_BITS: u64 = 41;

    pub fn new(lat: f64, lon: f64, time: Option<DateTime<Utc>>) -> Result<Self, &'static str> {
        if lat < -90.0 || lat > 90.0 {
            return Err("Latitude must be between -90 and 90");
        }
        if lon < -180.0 || lon > 180.0 {
            return Err("Longitude must be between -180 and 180");
        }

        // 1. Prepare Data
        let utc = time.unwrap_or_else(Utc::now);
        // Time in seconds (32 bits).
        // Note: This matches standard Unix timestamp behavior (seconds since 1970).
        // It fits in u32 until year 2106.
        let ts_sec = utc.timestamp() as u32;

        // Normalize Lat (24 bits)
        let lat_normalized = (lat + 90.0) / 180.0;
        let lat_int = (lat_normalized * ((1 << Self::LAT_BITS) as f64 - 1.0)).round() as u32;

        // Normalize Lon (25 bits)
        let lon_normalized = (lon + 180.0) / 360.0;
        let lon_int = (lon_normalized * ((1 << Self::LON_BITS) as f64 - 1.0)).round() as u32;

        // Random (41 bits)
        let mut rng = rand::rng();
        let rnd: u64 = rng.random_range(0..(1u64 << Self::RAND_BITS));

        // 2. Interleave Bits
        // Sources: Time(32), Lon(25), Lat(24).
        // Strategy: Round-robin MSB Interleaving (T, O, L).
        // Total T+O+L = 81 bits.
        // Followed by 41 bits of Random.
        // Total Payload = 122 bits.

        let mut uuid_u128: u128 = 0;

        // Flatten sources into a single 122-bit buffer first.
        let mut payload_bits = [false; 122];
        let mut pb_idx = 0;

        // Interleave T, O, L
        // Max iterations = 32 (max bits of Time).
        for i in (0..32).rev() {
            // T (32 bits)
            if i < 32 {
                payload_bits[pb_idx] = (ts_sec >> i) & 1 == 1;
                pb_idx += 1;
            }
            // O
            // Lon has 25 bits. Indices 24..0.
            // Alignment: T[31] (approx 68 years) aligns roughly to O[24] (20,000km).
            let idx_o = i as isize - (32 - 25);
            if idx_o >= 0 {
                payload_bits[pb_idx] = (lon_int >> idx_o) & 1 == 1;
                pb_idx += 1;
            }

            // L
            // Lat 24 bits. indices 23..0.
            let idx_l = i as isize - (32 - 24);
            if idx_l >= 0 {
                payload_bits[pb_idx] = (lat_int >> idx_l) & 1 == 1;
                pb_idx += 1;
            }
        }

        // Append Random (41 bits)
        // R indices 40..0
        for i in (0..41).rev() {
            payload_bits[pb_idx] = (rnd >> i) & 1 == 1;
            pb_idx += 1;
        }

        // Now pack into UUID
        let mut p_cursor = 0;
        for p in (0..128).rev() {
            let abs_pos = 127 - p;

            if (48..52).contains(&abs_pos) {
                 if matches!(abs_pos, 49 | 50 | 51) { uuid_u128 |= 1 << p; }
            } else if (64..66).contains(&abs_pos) {
                 if matches!(abs_pos, 64) { uuid_u128 |= 1 << p; }
            } else {
                if payload_bits[p_cursor] {
                    uuid_u128 |= 1 << p;
                }
                p_cursor += 1;
            }
        }

        Ok(GeoTemporalUuid(uuid_u128.to_be_bytes()))
    }

    pub fn to_uuid_string(&self) -> String {
        let b = &self.0;
        format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }

    pub fn decode(&self) -> (f64, f64, DateTime<Utc>) {
        let uuid_u128 = u128::from_be_bytes(self.0);

        let mut ts_sec: u32 = 0;
        let mut lat_int: u32 = 0;
        let mut lon_int: u32 = 0;

        // Recover payload bits
        let mut payload_bits = [false; 122];
        let mut p_cursor = 0;

        // Walk the UUID bits exactly as in new() to extract payload stream
        for p in (0..128).rev() {
            let abs_pos = 127 - p;
            if (48..52).contains(&abs_pos) || (64..66).contains(&abs_pos) {
                 continue;
            }

            let bit = (uuid_u128 >> p) & 1 == 1;
            payload_bits[p_cursor] = bit;
            p_cursor += 1;
        }

        // De-interleave payload_bits -> ts, lat, lon
        // Logic must strictly mirror new().

        let mut pb_idx = 0;
        for i in (0..32).rev() {
             // T
            if i < 32 {
                if payload_bits[pb_idx] {
                    ts_sec |= 1 << i;
                }
                pb_idx += 1;
            }
            // O
            let idx_o = i as isize - (32 - 25);
            if idx_o >= 0 {
                if payload_bits[pb_idx] {
                    lon_int |= 1 << idx_o;
                }
                pb_idx += 1;
            }
            // L
            let idx_l = i as isize - (32 - 24);
            if idx_l >= 0 {
                if payload_bits[pb_idx] {
                    lat_int |= 1 << idx_l;
                }
                pb_idx += 1;
            }
        }

        // Reconstruct float values
        let lat = (lat_int as f64 / ((1 << Self::LAT_BITS) as f64 - 1.0)) * 180.0 - 90.0;
        let lon = (lon_int as f64 / ((1 << Self::LON_BITS) as f64 - 1.0)) * 360.0 - 180.0;

        // Reconstruct time
        let time = Utc.timestamp_opt(ts_sec as i64, 0).unwrap();

        (lat, lon, time)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        GeoTemporalUuid(bytes)
    }
}

impl std::str::FromStr for GeoTemporalUuid {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.replace("-", "");
        let bytes = hex::decode(clean).map_err(|e| e.to_string())?;
        if bytes.len() != 16 {
            return Err("Invalid length".into());
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(GeoTemporalUuid(arr))
    }
}


impl fmt::Display for GeoTemporalUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uuid_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let lat = 40.6892;
        let lon = -74.0445;
        let uuid = GeoTemporalUuid::new(lat, lon, None).unwrap();

        let (d_lat, d_lon, _time) = uuid.decode();

        // Check precision (approx 1e-5 degrees)
        assert!((lat - d_lat).abs() < 1e-5);
        assert!((lon - d_lon).abs() < 1e-5);
    }

    #[test]
    fn test_ordering() {
        let u1 = GeoTemporalUuid::new(0.0, 0.0, Some(Utc.timestamp_millis_opt(1000).unwrap())).unwrap();
        let u2 = GeoTemporalUuid::new(0.0, 0.0, Some(Utc.timestamp_millis_opt(2000).unwrap())).unwrap();

        assert!(u1 < u2); // Time dominant
    }
}



// WASM Interface
#[wasm_bindgen]
pub fn generate_uuid(lat: f64, lon: f64, time_input: JsValue) -> Result<String, String> {
    let time = if time_input.is_null() || time_input.is_undefined() {
        Utc::now()
    } else if let Some(ms) = time_input.as_f64() {
         let secs = (ms / 1000.0) as i64;
         let nsecs = ((ms % 1000.0) * 1_000_000.0) as u32;
         Utc.timestamp_opt(secs, nsecs).unwrap()
    } else if let Some(s) = time_input.as_string() {
        if let Ok(ms) = s.parse::<i64>() {
            Utc.timestamp_millis_opt(ms).unwrap()
        } else {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                     // Try other formats?
                     // naive datetime + utc?
                     // chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
                     //    .map(|dt| DateTime::<Utc>::from_utc(dt, Utc))
                     Err("Invalid format")
                })
                .map_err(|_| "Invalid ISO timestamp format")?
        }
    } else {
        return Err("Invalid time argument. Expected number (ms), string (ISO/ms), null, or undefined.".to_string());
    };

    let uuid = GeoTemporalUuid::new(lat, lon, Some(time)).map_err(|e| e.to_string())?;
    Ok(uuid.to_uuid_string())
}

#[wasm_bindgen]
pub fn decode_uuid(uuid_str: &str) -> Result<Box<[f64]>, String> {
    // Parse string manually since we don't have FromStr yet or helper
    // Easier to rely on hex parsing or implement logic.
    // Wait, we don't have a from_string method yet.

    use std::str::FromStr;
    let uuid = GeoTemporalUuid::from_str(uuid_str)?;

    let (lat, lon, time) = uuid.decode();
    let time_ms = time.timestamp_millis() as f64;

    // Return array: [lat, lon, time_ms]
    let res = Box::new([lat, lon, time_ms]);
    Ok(res)
}
