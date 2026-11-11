#!/usr/bin/env bash
# Uninstall Xenapse file type definitions and icons for the current user.
set -euo pipefail

TARGET_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
MANIFEST_DIR="${TARGET_DATA_DIR}/xenapse"
MANIFEST_FILE="${MANIFEST_DIR}/filetypes.manifest"

CUR_TMP_MANIFEST=""

cleanup_tmp_manifest() {
  if [ -n "${CUR_TMP_MANIFEST:-}" ] && [ -f "${CUR_TMP_MANIFEST}" ]; then
    rm -f "${CUR_TMP_MANIFEST}" 2>/dev/null || true
    CUR_TMP_MANIFEST=""
  fi
}

handle_signal() {
  cleanup_tmp_manifest
  trap - "$1"
  kill -s "$1" "$$"
}

trap cleanup_tmp_manifest EXIT
trap 'handle_signal INT' INT
trap 'handle_signal TERM' TERM
trap 'handle_signal HUP' HUP

echo "Uninstalling Xenapse file type definitions and icons from ${TARGET_DATA_DIR}..."

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
  echo "Error: TARGET_DATA_DIR '${TARGET_DATA_DIR}' is a symlink. Aborting uninstallation to prevent unsafe deletion." >&2
  exit 1
fi

if has_symlink_component "${MANIFEST_DIR}" || has_symlink_component "${MANIFEST_FILE}"; then
  echo "Error: Manifest directory or file is a symlink or contains symlink components. Aborting uninstallation to prevent unsafe deletion." >&2
  exit 1
fi

# Remove only files recorded in the Xenapse installation manifest matching the allowlist.
# Abort safely if the manifest is invalid or malformed to prevent unsafe deletion.
if [ -f "${MANIFEST_FILE}" ]; then
  files_to_remove=()
  while IFS= read -r file || [ -n "$file" ]; do
    [ -n "$file" ] || continue
    if ! is_allowed_path "$file"; then
      echo "Error: Manifest at '${MANIFEST_FILE}' contains invalid or out-of-scope entry: '$file'. Aborting uninstallation to prevent unsafe deletion." >&2
      exit 1
    fi
    # Preflight check every manifest entry and parent directory component before deleting any files
    if has_symlink_component "$file"; then
      echo "Error: Entry '$file' or a parent directory component is a symlink. Aborting uninstallation without deleting any files." >&2
      exit 1
    fi
    files_to_remove+=("$file")
  done < "${MANIFEST_FILE}"

  remaining_files=()
  for file in "${files_to_remove[@]}"; do
    if [ -e "$file" ] || [ -L "$file" ]; then
      if ! rm -f "$file"; then
        echo "Warning: Failed to remove '$file'." >&2
      fi
      if [ -e "$file" ] || [ -L "$file" ]; then
        remaining_files+=("$file")
      fi
    fi
  done

  if [ ${#remaining_files[@]} -gt 0 ]; then
    echo "Warning: Some tracked files could not be removed. Preserving manifest with remaining entries." >&2
    CUR_TMP_MANIFEST="${MANIFEST_DIR}/filetypes.manifest.tmp.$$.$RANDOM"
    if printf "%s\n" "${remaining_files[@]}" > "$CUR_TMP_MANIFEST" && mv -f "$CUR_TMP_MANIFEST" "${MANIFEST_FILE}"; then
      CUR_TMP_MANIFEST=""
    else
      cleanup_tmp_manifest
    fi
  else
    rm -f "${MANIFEST_FILE}"
    rmdir "${MANIFEST_DIR}" 2>/dev/null || true
  fi
else
  echo "Notice: No Xenapse install manifest found at ${MANIFEST_FILE}. Preserving all existing files."
fi

ICONS_DEST_DIR="${TARGET_DATA_DIR}/icons/hicolor"

# 3. Refresh MIME database
if command -v update-mime-database >/dev/null 2>&1; then
  echo "Updating MIME database..."
  update-mime-database "${TARGET_DATA_DIR}/mime" || true
else
  echo "Notice: 'update-mime-database' not found. MIME database cache was not refreshed."
fi

# 4. Refresh icon caches
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  if [ -d "${ICONS_DEST_DIR}" ]; then
    echo "Updating GTK icon cache..."
    gtk-update-icon-cache -q -t -f "${ICONS_DEST_DIR}" || true
  fi
else
  echo "Notice: 'gtk-update-icon-cache' not found. GTK icon cache was not refreshed."
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

echo "Uninstallation complete."
