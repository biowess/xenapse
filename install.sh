#!/usr/bin/env bash
# Xenapse Linux installer
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

if [[ "${EUID}" -eq 0 ]]; then
    printf 'Please run this installer as your normal user, without sudo.\n' >&2
    exit 1
fi

for cmd in cp mv ln mkdir rm mktemp readlink; do
    command -v "$cmd" >/dev/null 2>&1 || {
        printf 'Missing required command: %s\n' "$cmd" >&2
        exit 1
    }
done

[[ -x "$SCRIPT_DIR/xenapse" ]] || {
    printf 'Xenapse executable not found beside install.sh.\n' >&2
    exit 1
}

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
XENAPSE_HOME="$DATA_HOME/xenapse"
APP_DIR="$XENAPSE_HOME/app"
STAGE_DIR=""
BACKUP_DIR=""
LINK_TMP_DIR=""
APP_REPLACED=0
INSTALL_COMMITTED=0

# Respect NO_COLOR and non-interactive output.
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    TEAL=$'\033[38;2;141;181;173m'
    OXIDE=$'\033[38;2;209;139;108m'
    SLATE=$'\033[38;2;137;148;156m'
    IVORY=$'\033[38;2;242;240;233m'
    RESET=$'\033[0m'
else
    TEAL='' OXIDE='' SLATE='' IVORY='' RESET=''
fi

ok()   { printf '%s[ OK ]%s %s\n' "$TEAL" "$RESET" "$*"; }
warn() { printf '%s[WARN]%s %s\n' "$OXIDE" "$RESET" "$*"; }
fail() { printf '%s[FAIL]%s %s\n' "$OXIDE" "$RESET" "$*" >&2; }

cleanup() {
    local status=$?
    trap - EXIT

    if [[ -n "$STAGE_DIR" && -d "$STAGE_DIR" ]]; then
        rm -rf -- "$STAGE_DIR" || true
    fi

    if [[ -n "$LINK_TMP_DIR" && -d "$LINK_TMP_DIR" ]]; then
        rm -rf -- "$LINK_TMP_DIR" || true
    fi

    if (( ! INSTALL_COMMITTED )); then
        if [[ -n "$BACKUP_DIR" && -d "$BACKUP_DIR/app" ]]; then
            if (( APP_REPLACED )) && [[ -d "$APP_DIR" ]]; then
                rm -rf -- "$APP_DIR" || true
            fi

            if [[ ! -e "$APP_DIR" && ! -L "$APP_DIR" ]]; then
                if ! mv -- "$BACKUP_DIR/app" "$APP_DIR"; then
                    warn "Could not restore the previous installation. Backup preserved at: $BACKUP_DIR"
                fi
            fi
        elif (( APP_REPLACED )) && [[ -d "$APP_DIR" ]]; then
            rm -rf -- "$APP_DIR" || true

            if [[ -L "$BIN_DIR/xenapse" ]] &&
               [[ "$(readlink "$BIN_DIR/xenapse")" == "$APP_DIR/xenapse" ]]; then
                rm -f -- "$BIN_DIR/xenapse" || true
            fi
        fi
    fi

    if (( INSTALL_COMMITTED )); then
        [[ -n "$BACKUP_DIR" && -d "$BACKUP_DIR" ]] &&
            rm -rf -- "$BACKUP_DIR"
    elif [[ -n "$BACKUP_DIR" && -d "$BACKUP_DIR" &&
            ! -e "$BACKUP_DIR/app" ]]; then
        rm -rf -- "$BACKUP_DIR"
    fi

    exit "$status"
}
trap cleanup EXIT

ask() {
    local answer
    while true; do
        printf '%s%s [y/n]: %s' "$TEAL" "$1" "$RESET"
        read -r answer
        case "${answer,,}" in
            y|yes) return 0 ;;
            n|no|'') return 1 ;;
            *) printf 'Please enter y or n.\n' ;;
        esac
    done
}

printf '%s' "$TEAL"
cat <<'LOGO'
██╗  ██╗███████╗███╗   ██╗ █████╗ ██████╗ ███████╗███████╗
╚██╗██╔╝██╔════╝████╗  ██║██╔══██╗██╔══██╗██╔════╝██╔════╝
 ╚███╔╝ █████╗  ██╔██╗ ██║███████║██████╔╝███████╗█████╗
 ██╔██╗ ██╔══╝  ██║╚██╗██║██╔══██║██╔═══╝ ╚════██║██╔══╝
██╔╝ ██╗███████╗██║ ╚████║██║  ██║██║     ███████║███████╗
╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝     ╚══════╝╚══════╝
LOGO
printf '%s\n' "$RESET"

printf '%sXenapse Installer%s\n' "$IVORY" "$RESET"
printf '%sWelcome to Xenapse.%s\n\n' "$TEAL" "$RESET"
printf 'This wizard will install the Xenapse compiler for your user account.\n'
printf 'No administrator privileges are required.\n\n'

printf '%sINSTALLATION CONFIGURATION%s\n' "$SLATE" "$RESET"
printf 'Executable: %s\n' "$BIN_DIR/xenapse"
printf 'Application: %s\n\n' "$APP_DIR"

REGISTER_TYPES=0
if ask "Register .xnp and .xcss file types with your desktop environment?"; then
    REGISTER_TYPES=1
fi

printf '\n%sINSTALLATION SUMMARY%s\n' "$SLATE" "$RESET"
printf '  Application: %s\n' "$APP_DIR"
printf '  Executable:  %s\n' "$BIN_DIR/xenapse"
if (( REGISTER_TYPES )); then
    printf '  File types:  Register .xnp and .xcss\n'
else
    printf '  File types:  Skip registration\n'
fi

printf '\n'
if ! ask "Proceed with installation?"; then
    printf '\nInstallation cancelled.\n'
    exit 0
fi

printf '\n%sINSTALLING XENAPSE%s\n' "$TEAL" "$RESET"

mkdir -p "$XENAPSE_HOME" "$BIN_DIR"

# Never overwrite an application directory not marked as Xenapse-owned.
if [[ -e "$APP_DIR" || -L "$APP_DIR" ]]; then
    if [[ -L "$APP_DIR" || ! -f "$APP_DIR/.xenapse-install" ]] ||
       [[ "$(cat "$APP_DIR/.xenapse-install" 2>/dev/null || true)" != "XENAPSE" ]]; then
        fail "Existing application directory is not recognized as a Xenapse installation."
        fail "Preserving: $APP_DIR"
        exit 1
    fi
fi

# Never replace an unrelated executable or symlink.
if [[ -e "$BIN_DIR/xenapse" || -L "$BIN_DIR/xenapse" ]]; then
    if [[ ! -L "$BIN_DIR/xenapse" ]] ||
       [[ "$(readlink "$BIN_DIR/xenapse")" != "$APP_DIR/xenapse" ]]; then
        fail "An unrelated file already exists at $BIN_DIR/xenapse."
        fail "Please move it before installing Xenapse."
        exit 1
    fi
fi

STAGE_DIR="$(mktemp -d "$XENAPSE_HOME/.app-stage.XXXXXX")"

cp "$SCRIPT_DIR/xenapse" "$STAGE_DIR/xenapse"
cp "$SCRIPT_DIR/README.md" "$STAGE_DIR/README.md"
cp "$SCRIPT_DIR/LICENSE" "$STAGE_DIR/LICENSE"
cp "$SCRIPT_DIR/NOTICE" "$STAGE_DIR/NOTICE"
cp "$SCRIPT_DIR/install.sh" "$STAGE_DIR/install.sh"
cp "$SCRIPT_DIR/uninstall.sh" "$STAGE_DIR/uninstall.sh"
cp -R "$SCRIPT_DIR/packaging" "$STAGE_DIR/packaging"
chmod 755 "$STAGE_DIR/install.sh" "$STAGE_DIR/uninstall.sh"
chmod 755 "$STAGE_DIR/xenapse"
printf 'XENAPSE\n' > "$STAGE_DIR/.xenapse-install"

# Preserve the previous installation until the new one is committed.
if [[ -d "$APP_DIR" ]]; then
    BACKUP_DIR="$(mktemp -d "$XENAPSE_HOME/.app-backup.XXXXXX")"
    mv -- "$APP_DIR" "$BACKUP_DIR/app"
fi

APP_REPLACED=1
mv -- "$STAGE_DIR" "$APP_DIR"
STAGE_DIR=""

# Atomically create or refresh the executable link using a unique temp directory.
LINK_TMP_DIR="$(mktemp -d "$BIN_DIR/.xenapse-link.XXXXXX")"
ln -s "$APP_DIR/xenapse" "$LINK_TMP_DIR/xenapse"
mv -f -- "$LINK_TMP_DIR/xenapse" "$BIN_DIR/xenapse"
rmdir "$LINK_TMP_DIR"
LINK_TMP_DIR=""

INSTALL_COMMITTED=1

ok "Compiler installed"
ok "Executable linked"

# File association registration is optional and reuses the existing scripts.
if (( REGISTER_TYPES )); then
    printf '\nRegistering desktop file types...\n'
    if bash "$APP_DIR/packaging/install-filetypes.sh"; then
        ok "File types registered"
    else
        warn "File type registration failed. The compiler itself is installed."
        warn "You can retry later with: bash \"$APP_DIR/packaging/install-filetypes.sh\""
    fi
else
    printf '%s[SKIP]%s File type registration skipped\n' "$SLATE" "$RESET"
fi

printf '\n%sINSTALLATION COMPLETE%s\n' "$TEAL" "$RESET"
printf 'Run Xenapse with: %sxenapse --help%s\n' "$TEAL" "$RESET"

case ":${PATH}:" in
    *":$BIN_DIR:"*) ;;
    *)
        warn "$BIN_DIR is not currently in your PATH."
        printf 'Add this line to your shell configuration:\n'
        printf '  export PATH="$HOME/.local/bin:$PATH"\n'
        ;;
esac

printf '\nTo uninstall Xenapse later, run:\n'
printf '  bash "%s/uninstall.sh"\n' "$APP_DIR"
