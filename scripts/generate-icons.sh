#!/usr/bin/env bash
# Generate Linux freedesktop icon hierarchy and Windows multi-resolution .ico files
# from the source SVGs in assets/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSETS_DIR="${REPO_ROOT}/assets"
LINUX_ICONS_DIR="${REPO_ROOT}/packaging/linux/icons/hicolor"
WIN_ICONS_DIR="${REPO_ROOT}/packaging/windows/icons"

echo "Generating icon assets from ${ASSETS_DIR}..."

# Check prerequisites
for tool in rsvg-convert magick; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Error: required tool '$tool' not found." >&2
    exit 1
  fi
done

# Prepare Linux icon theme hierarchy
mkdir -p "${LINUX_ICONS_DIR}/scalable/mimetypes"
mkdir -p "${LINUX_ICONS_DIR}/symbolic/mimetypes"
mkdir -p "${WIN_ICONS_DIR}"

# 1. Linux Scalable SVGs (full color)
cp "${ASSETS_DIR}/xnp.svg" "${LINUX_ICONS_DIR}/scalable/mimetypes/application-vnd.xenapse.diagram.svg"
cp "${ASSETS_DIR}/xcss.svg" "${LINUX_ICONS_DIR}/scalable/mimetypes/text-vnd.xenapse.stylesheet.svg"

# 2. Linux Symbolic SVGs (monochrome)
cp "${ASSETS_DIR}/xnp-mono.svg" "${LINUX_ICONS_DIR}/symbolic/mimetypes/application-vnd.xenapse.diagram-symbolic.svg"
cp "${ASSETS_DIR}/xcss-mono.svg" "${LINUX_ICONS_DIR}/symbolic/mimetypes/text-vnd.xenapse.stylesheet-symbolic.svg"

# 3. Linux PNG sizes and Windows multi-resolution ICOs
SIZES=(16 24 32 48 64 128 256)
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

for target in xnp xcss xnp-mono xcss-mono; do
  svg_src="${ASSETS_DIR}/${target}.svg"
  png_list=()

  for s in "${SIZES[@]}"; do
    png_out="${TMP_DIR}/${target}_${s}.png"
    rsvg-convert -w "$s" -h "$s" -f png "$svg_src" -o "$png_out"
    png_list+=("$png_out")

    # For Linux, install colored variants into hicolor/<size>x<size>/mimetypes
    if [ "$target" = "xnp" ]; then
      mkdir -p "${LINUX_ICONS_DIR}/${s}x${s}/mimetypes"
      cp "$png_out" "${LINUX_ICONS_DIR}/${s}x${s}/mimetypes/application-vnd.xenapse.diagram.png"
    elif [ "$target" = "xcss" ]; then
      mkdir -p "${LINUX_ICONS_DIR}/${s}x${s}/mimetypes"
      cp "$png_out" "${LINUX_ICONS_DIR}/${s}x${s}/mimetypes/text-vnd.xenapse.stylesheet.png"
    fi
  done

  # Build multi-resolution ICO for Windows
  ico_out="${WIN_ICONS_DIR}/${target}.ico"
  magick "${png_list[@]}" "$ico_out"
  echo "Generated: ${ico_out}"
done

echo "Icon generation complete."
