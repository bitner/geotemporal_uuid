#!/bin/bash
set -e

# Build the WASM package directly into the web directory
echo "Building WASM..."
wasm-pack build --target web --out-dir www/pkg

# Serve
echo "Serving at http://localhost:8000"
cd www
python3 -m http.server
