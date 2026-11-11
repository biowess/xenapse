#!/usr/bin/env bash
# Package Xenapse release archive for Linux x86_64.
# Run from repository root.
set -euo pipefail

VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c '
import json, sys
packages = [p for p in json.load(sys.stdin)["packages"] if p["name"] == "xenapse-cli"]
if len(packages) != 1:
    raise SystemExit("Expected exactly one xenapse-cli package")
print(packages[0]["version"])
')"
TARGET_NAME="linux-x86_64"
ARCHIVE_NAME="xenapse-${VERSION}-${TARGET_NAME}"
DIST_DIR="dist/${ARCHIVE_NAME}"

# Safety check: refuse to proceed if dist/ is a symbolic link or redirection
if [ -L "dist" ]; then
    echo "Error: 'dist' already exists as a symbolic link. Refusing to proceed." >&2
    exit 1
fi

# Build release binary natively
cargo build --release -p xenapse-cli

# Prepare staging directory (clean only Xenapse staging dir and exact release files)
rm -rf "${DIST_DIR}"
rm -f "dist/${ARCHIVE_NAME}.tar.gz" "dist/${ARCHIVE_NAME}.tar.gz.sha256"
mkdir -p "${DIST_DIR}"

# Copy binary, license, notice, and documentation
cp target/release/xenapse "${DIST_DIR}/"
cp LICENSE "${DIST_DIR}/"
cp NOTICE "${DIST_DIR}/"
cp README.md "${DIST_DIR}/"

# Copy file type integration assets and installation scripts
mkdir -p "${DIST_DIR}/packaging"
cp -r packaging/linux/* "${DIST_DIR}/packaging/"

# Create compressed tarball
tar -czf "dist/${ARCHIVE_NAME}.tar.gz" -C dist "${ARCHIVE_NAME}"

# Generate SHA-256 checksum
(cd dist && sha256sum "${ARCHIVE_NAME}.tar.gz" > "${ARCHIVE_NAME}.tar.gz.sha256")

echo "Packaging complete: dist/${ARCHIVE_NAME}.tar.gz"
echo "Checksum: $(cat dist/${ARCHIVE_NAME}.tar.gz.sha256)"
