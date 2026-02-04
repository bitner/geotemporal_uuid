# GeoTemporal UUID

A Rust library, CLI, and WASM package for generating UUIDs that are sortable by both **Time** and **Location** (Latitude/Longitude).

![Test Status](https://github.com/USERNAME/REPO/actions/workflows/ci.yml/badge.svg)

## Concept

Standard UUIDv7 is sortable by time. `GeoTemporalUuid` extends this by interleaving spatial data (Latitude/Longitude) with Time data using a **Z-order (Morton) Curve**.

This means that UUIDs generated near the same time and place will be numerically closer to each other than those generated far apart. This clustering property is excellent for database indexing of spatiotemporal data.

### Bit Layout (128 bits)

*   **Version:** 7 (Custom layout compatible with UUIDv7 parsers)
*   **Variant:** 10 (Standard RFC 4122)
*   **Payload Construction:**
    *   **Tier 1 (Bits 0-80 roughly):** A Z-Curve interleaving of:
        *   Time (32 bits, Seconds precision)
        *   Latitude (24 bits, ~1.2m precision)
        *   Longitude (25 bits, ~1.2m precision)
    *   **Tier 2 (Bits 81-121):** Randomness (41 bits) to ensure uniqueness.

The interleaving starts from the most significant bit, ensuring effective multidimensional sorting.

### Precision & Collision Resistance

**Temporal Precision:**
*   32 bits are dedicated to validity in **seconds** (Unix Epoch).
*   Correctness is maintained to the **second**.
*   Range: Valid through year 2106 AD.

**Spatial Precision:**
*   **Latitude:** 24 bits (~1.19 meters).
*   **Longitude:** 25 bits (~1.19 meters at equator).
*   Points within this grid are considered strictly identical spatially.

**Collision Likelihood:**
*   Collisions can only occur if two IDs are generated at the **exact same second** AND within the **same 1.2m² grid cell**.
*   Even then, there are **41 bits of randomness** (~2.2 trillion values) to distinguish them.
*   By the Birthday Paradox, you would need to generate ~1.5 million IDs *per second* in the *same 1 meter spot* to have a 50% chance of collision.
*   This provides extremely high collision resistance for distributed systems.

## Usage

### Rust Library

Add to `Cargo.toml`:
```toml
[dependencies]
geotemporal_uuid = { git = "https://github.com/USERNAME/REPO" }
```

```rust
use geotemporal_uuid::GeoTemporalUuid;

fn main() {
    let lat = 37.7749;
    let lon = -122.4194;

    // Generate
    let uuid = GeoTemporalUuid::new(lat, lon, None).unwrap();
    println!("{}", uuid);

    // Decode
    let (d_lat, d_lon, d_time) = uuid.decode();
}
```

### CLI Tool

Clone and run:

```bash
# Generate (Current time)
cargo run -- generate --lat 40.7128 --lon -74.0060

# Generate (Specific time, ISO-8601 or Milliseconds)
cargo run -- generate --lat 40.7128 --lon -74.0060 --time "2023-01-01T12:00:00Z"

# Decode
cargo run -- decode <UUID_STRING>
```

### WebAssembly (WASM) & Demo

This project includes a WASM build target and a demo web page.
To run it locally with the correct paths:

```bash
# 1. Build into the www/pkg directory
wasm-pack build --target web --out-dir www/pkg

# 2. Serve from the www directory
cd www
python3 -m http.server
# Open browser to http://localhost:8000
```

Or simply run the helper script:

```bash
./serve_demo.sh
```

## License

MIT
