#!/usr/bin/env bash
# Install Xenapse file type definitions and icons for the current user.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
MANIFEST_DIR="${TARGET_DATA_DIR}/xenapse"
MANIFEST_FILE="${MANIFEST_DIR}/filetypes.manifest"

CUR_TMP_MANIFEST=""
CUR_TMP_FILE=""

cleanup_tmp_files() {
  if [ -n "${CUR_TMP_MANIFEST:-}" ] && [ -f "${CUR_TMP_MANIFEST}" ]; then
    rm -f "${CUR_TMP_MANIFEST}" 2>/dev/null || true
    CUR_TMP_MANIFEST=""
  fi
  if [ -n "${CUR_TMP_FILE:-}" ] && [ -f "${CUR_TMP_FILE}" ]; then
    rm -f "${CUR_TMP_FILE}" 2>/dev/null || true
    CUR_TMP_FILE=""
  fi
}

handle_signal() {
  cleanup_tmp_files
  trap - "$1"
  kill -s "$1" "$$"
}

trap cleanup_tmp_files EXIT
trap 'handle_signal INT' INT
trap 'handle_signal TERM' TERM
trap 'handle_signal HUP' HUP

echo "Installing Xenapse file type definitions and icons to ${TARGET_DATA_DIR}..."

is_allowed_path() {
  local p="$1"
  case "$p" in
    "${TARGET_DATA_DIR}/mime/packages/xenapse.xml") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/scalable/mimetypes/application-vnd.xenapse.diagram.svg") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/scalable/mimetypes/text-vnd.xenapse.stylesheet.svg") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/symbolic/mimetypes/application-vnd.xenapse.diagram-symbolic.svg") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/symbolic/mimetypes/text-vnd.xenapse.stylesheet-symbolic.svg") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/16x16/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/16x16/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/24x24/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/24x24/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/32x32/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/32x32/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/48x48/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/48x48/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/64x64/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/64x64/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/128x128/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/128x128/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/256x256/mimetypes/application-vnd.xenapse.diagram.png") return 0 ;;
    "${TARGET_DATA_DIR}/icons/hicolor/256x256/mimetypes/text-vnd.xenapse.stylesheet.png") return 0 ;;
    *) return 1 ;;
  esac
}

has_symlink_component() {
  local target="$1"
  # Security boundary: only check path components from $target down to TARGET_DATA_DIR
  # (inclusive). Parent directories above TARGET_DATA_DIR are outside Xenapse's scope;
  # those are trusted by the operating system and XDG conventions and are not checked here.
  if [ -L "${TARGET_DATA_DIR}" ]; then
    return 0
  fi
  local cur="$target"
  while [ "$cur" != "/" ] && [ "$cur" != "." ]; do
    if [ -L "$cur" ]; then
      return 0
    fi
    if [ "$cur" = "$TARGET_DATA_DIR" ]; then
      break
    fi
    cur="$(dirname "$cur")"
  done
  return 1
}

if [ -L "${TARGET_DATA_DIR}" ]; then
  echo "Error: TARGET_DATA_DIR '${TARGET_DATA_DIR}' is a symlink. Aborting." >&2
  exit 1
fi

if has_symlink_component "${MANIFEST_DIR}" || has_symlink_component "${MANIFEST_FILE}"; then
  echo "Error: Manifest directory or file is a symlink or contains symlink components. Aborting to prevent ownership loss." >&2
  exit 1
fi

# Load prior Xenapse install manifest to allow safe repeated installs.
# Abort safely if manifest is malformed to avoid discarding valid ownership information.
installed_files=()
if [ -f "${MANIFEST_FILE}" ]; then
  while IFS= read -r line || [ -n "$line" ]; do
    [ -n "$line" ] || continue
    if ! is_allowed_path "$line"; then
      echo "Error: Manifest at '${MANIFEST_FILE}' contains invalid or out-of-scope entry: '$line'. Aborting to prevent ownership loss." >&2
      exit 1
    fi
    installed_files+=("$line")
  done < "${MANIFEST_FILE}"
fi

is_owned_by_xenapse() {
  local target="$1"
  for f in "${installed_files[@]}"; do
    if [ "$f" = "$target" ]; then
      return 0
    fi
  done
  return 1
}

tracked_manifest=("${installed_files[@]}")

persist_manifest() {
  if has_symlink_component "${MANIFEST_DIR}" || has_symlink_component "${MANIFEST_FILE}"; then
    echo "Error: Manifest directory or file is a symlink or contains symlink components. Refusing to write manifest." >&2
    return 1
  fi
  if [ ${#tracked_manifest[@]} -gt 0 ]; then
    mkdir -p "${MANIFEST_DIR}"
    CUR_TMP_MANIFEST="${MANIFEST_DIR}/filetypes.manifest.tmp.$$.$RANDOM"
    if printf "%s\n" "${tracked_manifest[@]}" > "$CUR_TMP_MANIFEST" && mv -f "$CUR_TMP_MANIFEST" "${MANIFEST_FILE}"; then
      CUR_TMP_MANIFEST=""
      return 0
    else
      if [ -n "${CUR_TMP_MANIFEST:-}" ] && [ -f "${CUR_TMP_MANIFEST}" ]; then
        rm -f "${CUR_TMP_MANIFEST}" 2>/dev/null || true
      fi
      CUR_TMP_MANIFEST=""
      return 1
    fi
  elif [ -f "${MANIFEST_FILE}" ]; then
    rm -f "${MANIFEST_FILE}"
    return 0
  fi
  return 0
}

install_file() {
  local src="$1"
  local dest="$2"

  if ! is_allowed_path "$dest"; then
    echo "Error: Target path '$dest' is outside allowed Xenapse destinations." >&2
    return 1
  fi

  # Refuse installation if destination or any parent directory component is a symlink
  if has_symlink_component "$dest"; then
    echo "Conflict: Destination or parent directory component for '$dest' is a symlink. Refusing installation." >&2
    return 0
  fi

  local dest_dir
  dest_dir="$(dirname "$dest")"

  if [ -e "$dest" ]; then
    if is_owned_by_xenapse "$dest"; then
      mkdir -p "$dest_dir"
      CUR_TMP_FILE="${dest}.tmp.$$.$RANDOM"
      if ! cp "$src" "$CUR_TMP_FILE"; then
        echo "Error: Failed to copy '$src' to temporary file '$CUR_TMP_FILE'." >&2
        rm -f "$CUR_TMP_FILE" 2>/dev/null || true
        CUR_TMP_FILE=""
        return 1
      fi
      if ! mv -f "$CUR_TMP_FILE" "$dest"; then
        echo "Error: Failed to atomically replace '$dest'." >&2
        rm -f "$CUR_TMP_FILE" 2>/dev/null || true
        CUR_TMP_FILE=""
        return 1
      fi
      CUR_TMP_FILE=""
      # Manifest is unchanged because $dest is already owned. Do not rewrite.
    else
      echo "Conflict: '$dest' already exists and was not installed by Xenapse. Preserving existing file." >&2
    fi
  else
    mkdir -p "$dest_dir"
    CUR_TMP_FILE="${dest}.tmp.$$.$RANDOM"
    if ! cp "$src" "$CUR_TMP_FILE"; then
      echo "Error: Failed to copy '$src' to temporary file '$CUR_TMP_FILE'." >&2
      rm -f "$CUR_TMP_FILE" 2>/dev/null || true
      CUR_TMP_FILE=""
      return 1
    fi
    if ! mv -f "$CUR_TMP_FILE" "$dest"; then
      echo "Error: Failed to create '$dest'." >&2
      rm -f "$CUR_TMP_FILE" 2>/dev/null || true
      CUR_TMP_FILE=""
      return 1
    fi
    CUR_TMP_FILE=""

    # Add newly created file and persist immediately; roll back if persistence fails
    tracked_manifest+=("$dest")
    if ! persist_manifest; then
      echo "Error: Failed to persist manifest for newly created '$dest'. Rolling back." >&2
      rm -f "$dest"
      local rolled_back=()
      for f in "${tracked_manifest[@]}"; do
        [ "$f" != "$dest" ] && rolled_back+=("$f")
      done
      tracked_manifest=("${rolled_back[@]}")
      return 1
    fi
  fi
}

# 1. Install shared-mime-info XML
MIME_PACKAGES_DIR="${TARGET_DATA_DIR}/mime/packages"
if [ -f "${SCRIPT_DIR}/xenapse.xml" ]; then
  install_file "${SCRIPT_DIR}/xenapse.xml" "${MIME_PACKAGES_DIR}/xenapse.xml"
fi

# 2. Install icons into the hicolor theme hierarchy
ICONS_SRC_DIR="${SCRIPT_DIR}/icons/hicolor"
ICONS_DEST_DIR="${TARGET_DATA_DIR}/icons/hicolor"

if [ -d "${ICONS_SRC_DIR}" ]; then
  # Scalable vector icons (full-color)
  if [ -d "${ICONS_SRC_DIR}/scalable/mimetypes" ]; then
    for icon in "${ICONS_SRC_DIR}/scalable/mimetypes/"*.svg; do
      [ -f "$icon" ] || continue
      install_file "$icon" "${ICONS_DEST_DIR}/scalable/mimetypes/$(basename "$icon")"
    done
  fi

  # Symbolic vector icons (monochrome)
  if [ -d "${ICONS_SRC_DIR}/symbolic/mimetypes" ]; then
    for icon in "${ICONS_SRC_DIR}/symbolic/mimetypes/"*.svg; do
      [ -f "$icon" ] || continue
      install_file "$icon" "${ICONS_DEST_DIR}/symbolic/mimetypes/$(basename "$icon")"
    done
  fi

  # Fixed PNG resolutions
  for size_dir in "${ICONS_SRC_DIR}"/*x*/mimetypes; do
    if [ -d "${size_dir}" ]; then
      rel_size="$(basename "$(dirname "${size_dir}")")"
      for icon in "${size_dir}/"*.png; do
        [ -f "$icon" ] || continue
        install_file "$icon" "${ICONS_DEST_DIR}/${rel_size}/mimetypes/$(basename "$icon")"
      done
    fi
  done
fi

# 3. Refresh MIME database
if command -v update-mime-database >/dev/null 2>&1; then
  echo "Updating MIME database..."
  update-mime-database "${TARGET_DATA_DIR}/mime" || true
else
  echo "Notice: 'update-mime-database' not found. MIME database cache was not refreshed."
fi

# 4. Refresh icon caches
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  echo "Updating GTK icon cache..."
  gtk-update-icon-cache -q -t -f "${ICONS_DEST_DIR}" || true
else
  echo "Notice: 'gtk-update-icon-cache' not found. GTK icon cache was not refreshed."
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

echo "Installation complete."
echo "Registered file types:"
echo "  - application/vnd.xenapse.diagram (*.xnp) -> Xenapse Diagram"
echo "  - text/vnd.xenapse.stylesheet (*.xcss)   -> Xenapse Stylesheet"
