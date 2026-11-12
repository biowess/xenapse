#!/usr/bin/env bash
# Xenapse Linux uninstaller
set -Eeuo pipefail

if [[ "${EUID}" -eq 0 ]]; then
    printf 'Please run this uninstaller as your normal user, without sudo.\n' >&2
    exit 1
fi

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
XENAPSE_HOME="$DATA_HOME/xenapse"
APP_DIR="$XENAPSE_HOME/app"
EXECUTABLE="$BIN_DIR/xenapse"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    TEAL=$'\033[38;2;141;181;173m'
    OXIDE=$'\033[38;2;209;139;108m'
    SLATE=$'\033[38;2;137;148;156m'
    RESET=$'\033[0m'
else
    TEAL='' OXIDE='' SLATE='' RESET=''
fi

printf '%sXenapse Uninstaller%s\n\n' "$TEAL" "$RESET"

if [[ ! -f "$APP_DIR/.xenapse-install" ]] ||
   [[ "$(cat "$APP_DIR/.xenapse-install" 2>/dev/null || true)" != "XENAPSE" ]]; then
    printf '%s[FAIL]%s No recognized Xenapse installation found.\n' "$OXIDE" "$RESET" >&2
    exit 1
fi

printf 'This will remove:\n'
printf '  %s\n' "$APP_DIR"
if [[ -L "$EXECUTABLE" ]] &&
   [[ "$(readlink "$EXECUTABLE")" == "$APP_DIR/xenapse" ]]; then
    printf '  %s\n' "$EXECUTABLE"
fi
printf '\n'

while true; do
    printf '%sUninstall Xenapse? [y/n]: %s' "$TEAL" "$RESET"
    read -r answer
    case "${answer,,}" in
        y|yes) break ;;
        n|no|'') printf 'Uninstallation cancelled.\n'; exit 0 ;;
        *) printf 'Please enter y or n.\n' ;;
    esac
done

# Remove registered file types only when an ownership manifest exists.
if [[ -f "$XENAPSE_HOME/filetypes.manifest" ]]; then
    if [[ -x "$APP_DIR/packaging/uninstall-filetypes.sh" ]]; then
        printf '\nRemoving registered file types...\n'
        bash "$APP_DIR/packaging/uninstall-filetypes.sh" || {
            printf '%s[WARN]%s File type cleanup reported an error.\n' "$OXIDE" "$RESET"
        }
    else
        printf '%s[WARN]%s File type manifest exists, but cleanup script is missing.\n' "$OXIDE" "$RESET"
    fi
fi

if [[ -L "$EXECUTABLE" ]] &&
   [[ "$(readlink "$EXECUTABLE")" == "$APP_DIR/xenapse" ]]; then
    rm -- "$EXECUTABLE"
    printf '%s[ OK ]%s Executable link removed\n' "$TEAL" "$RESET"
elif [[ -e "$EXECUTABLE" || -L "$EXECUTABLE" ]]; then
    printf '%s[WARN]%s Executable path has changed; preserving it.\n' "$OXIDE" "$RESET"
fi

rm -rf -- "$APP_DIR"
printf '%s[ OK ]%s Xenapse application files removed\n' "$TEAL" "$RESET"

rmdir "$XENAPSE_HOME" 2>/dev/null || true

printf '\n%sXenapse has been uninstalled.%s\n' "$TEAL" "$RESET"
printf 'Your projects and source files have not been touched.\n'
