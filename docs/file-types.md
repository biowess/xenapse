# Xenapse Operating System File Type Integration

This document specifies the native operating-system file type registrations, icon assets, and integration procedures for the Xenapse language distribution on Linux and Windows.

---

## 1. Supported File Types

Xenapse defines two distinct file types:

| File Extension | Display Name | MIME Type / Content Type | Windows ProgID | Perceived Type |
|---|---|---|---|---|
| `.xnp` | **Xenapse Diagram** | `application/vnd.xenapse.diagram` | `Xenapse.Diagram` | `document` |
| `.xcss` | **Xenapse Stylesheet** | `text/vnd.xenapse.stylesheet` | `Xenapse.Stylesheet` | `text` |

- **Xenapse Diagram (`.xnp`)**: Source document for diagram definition (using modules such as `FLOW` and `MINDMAP`).
- **Xenapse Stylesheet (`.xcss`)**: Style definitions referenced by or applied to Xenapse diagrams (subclassed from `text/plain`).

> **Note on Graphical Editor**: Registering file types does **not** install a graphical editor or configure the command-line interface (`xenapse`) as the default application. The CLI compiler does not provide graphical editing, double-click handling, or interactive viewer functionality. Application association is left to the user or future tools such as **Xenapse Studio**.

---

## 2. Icon Assets

The Xenapse distribution provides both full-color and monochrome vector artwork in `assets/`:
- `xnp.svg` / `xnp-mono.svg`: Xenapse Diagram icons.
- `xcss.svg` / `xcss-mono.svg`: Xenapse Stylesheet icons.

### Linux Icon Hierarchy
Linux installations follow the standard [freedesktop.org Icon Theme Specification](https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html) under the `hicolor` base theme:
- **Full-color Scalable Vector**:
  - `scalable/mimetypes/application-vnd.xenapse.diagram.svg`
  - `scalable/mimetypes/text-vnd.xenapse.stylesheet.svg`
- **Monochrome Symbolic Vector** (for GNOME/KDE symbolic icon themes):
  - `symbolic/mimetypes/application-vnd.xenapse.diagram-symbolic.svg`
  - `symbolic/mimetypes/text-vnd.xenapse.stylesheet-symbolic.svg`
- **Bitmap Resolutions (PNG)**:
  - Sizes: `16x16`, `24x24`, `32x32`, `48x48`, `64x64`, `128x128`, `256x256`
  - Placed under `<size>/mimetypes/application-vnd.xenapse.diagram.png` and `<size>/mimetypes/text-vnd.xenapse.stylesheet.png`.

### Windows Icons
Windows installations use multi-resolution `.ico` files containing 7 embedded mipmap resolutions (`16x16`, `24x24`, `32x32`, `48x48`, `64x64`, `128x128`, and `256x256`) with 32-bit RGBA color and alpha transparency:
- `xnp.ico`: Full-color Xenapse Diagram icon.
- `xcss.ico`: Full-color Xenapse Stylesheet icon.
- `xnp-mono.ico` & `xcss-mono.ico`: Monochrome variants preserved for future high-contrast or specialized display modes.

---

## 3. Linux Integration

Linux integration complies with the [freedesktop.org Shared MIME-info](https://specifications.freedesktop.org/shared-mime-info-spec/shared-mime-info-spec-latest.html) and Icon Theme specifications. All operations support per-user execution without root permissions.

### Installation Path
- **MIME Definition**: `$XDG_DATA_HOME/mime/packages/xenapse.xml` (defaults to `~/.local/share/mime/packages/xenapse.xml`)
- **Icon Directory**: `$XDG_DATA_HOME/icons/hicolor/` (defaults to `~/.local/share/icons/hicolor/`)

### What the Install Script Does (`packaging/linux/install-filetypes.sh`)
1. Copies `xenapse.xml` to `$XDG_DATA_HOME/mime/packages/`.
2. Copies scalable SVGs, symbolic SVGs, and PNG resolutions into `$XDG_DATA_HOME/icons/hicolor/`.
3. Runs `update-mime-database` to reindex MIME types (if installed).
4. Runs `gtk-update-icon-cache` and `kbuildsycoca` to reindex icon caches (if installed).
5. Does not associate any application launcher or modify default openers.

### What the Uninstall Script Does (`packaging/linux/uninstall-filetypes.sh`)
1. Removes `$XDG_DATA_HOME/mime/packages/xenapse.xml`.
2. Removes only the installed Xenapse SVG and PNG icons from `$XDG_DATA_HOME/icons/hicolor/`.
3. Cleans up empty subdirectories left behind without modifying unrelated icons.
4. Refreshes the MIME and icon caches.

### Commands
```bash
# Install (per-user)
bash packaging/linux/install-filetypes.sh

# Uninstall (per-user)
bash packaging/linux/uninstall-filetypes.sh
```

---

## 4. Windows Integration

Windows integration utilizes user-scoped ProgIDs in the Windows Registry (`HKEY_CURRENT_USER\Software\Classes`). No administrative privileges or elevation are required.

### Registry Structure
- **ProgIDs**:
  - `HKCU\Software\Classes\Xenapse.Diagram`:
    - `(Default)`: `"Xenapse Diagram"`
    - `FriendlyTypeName`: `"Xenapse Diagram"`
    - `DefaultIcon`: `"<IconDir>\xnp.ico,0"`
  - `HKCU\Software\Classes\Xenapse.Stylesheet`:
    - `(Default)`: `"Xenapse Stylesheet"`
    - `FriendlyTypeName`: `"Xenapse Stylesheet"`
    - `DefaultIcon`: `"<IconDir>\xcss.ico,0"`
- **Extension Keys**:
  - `HKCU\Software\Classes\.xnp`:
    - `(Default)`: `"Xenapse.Diagram"` (only set if currently empty or already set to Xenapse)
    - `Content Type`: `"application/vnd.xenapse.diagram"`
    - `PerceivedType`: `"document"`
    - `OpenWithProgids`: `Xenapse.Diagram`
  - `HKCU\Software\Classes\.xcss`:
    - `(Default)`: `"Xenapse.Stylesheet"` (only set if currently empty or already set to Xenapse)
    - `Content Type`: `"text/vnd.xenapse.stylesheet"`
    - `PerceivedType`: `"text"`
    - `OpenWithProgids`: `Xenapse.Stylesheet`

### Icon Storage Location
By default, icons are installed to `%LOCALAPPDATA%\Xenapse\icons\` (e.g. `C:\Users\<user>\AppData\Local\Xenapse\icons\`). An alternate destination can be specified using the `-IconDir` parameter.

### What the Install Script Does (`packaging/windows/install-filetypes.ps1`)
1. Copies `xnp.ico`, `xcss.ico`, `xnp-mono.ico`, and `xcss-mono.ico` to the target icon directory.
2. Creates the ProgIDs `Xenapse.Diagram` and `Xenapse.Stylesheet` with friendly names and icon paths.
3. Maps `.xnp` and `.xcss` extensions to the ProgIDs, preserving any unrelated existing associations.
4. Calls `SHChangeNotify` via P/Invoke to notify Windows Explorer to refresh file icons immediately.
5. Does not register any `shell\open\command` entry.

### What the Uninstall Script Does (`packaging/windows/uninstall-filetypes.ps1`)
1. Clears Xenapse ProgID references from `.xnp` and `.xcss` keys and removes the keys if no unrelated values remain.
2. Deletes the `Xenapse.Diagram` and `Xenapse.Stylesheet` ProgID keys.
3. Deletes installed `.ico` files and removes the directory if empty.
4. Calls `SHChangeNotify` to refresh Windows Explorer.

### Commands
In PowerShell:
```powershell
# Install (per-user)
powershell -ExecutionPolicy Bypass -File .\packaging\windows\install-filetypes.ps1

# Uninstall (per-user)
powershell -ExecutionPolicy Bypass -File .\packaging\windows\uninstall-filetypes.ps1
```

---

## 5. Future Xenapse Studio Integration

Xenapse Studio will be released as a separate graphical editor. Because the language distribution registers standard MIME types and ProgIDs, Studio can integrate seamlessly without conflicting with existing definitions:

### On Linux
- Studio installs a `.desktop` file (e.g. `xenapse-studio.desktop`) declaring:
  ```desktop
  MimeType=application/vnd.xenapse.diagram;text/vnd.xenapse.stylesheet;
  ```
- Studio sets itself as the preferred handler via `xdg-mime default xenapse-studio.desktop application/vnd.xenapse.diagram text/vnd.xenapse.stylesheet`.
- The existing MIME XML (`application/vnd.xenapse.diagram`, `text/vnd.xenapse.stylesheet`) and icons are automatically reused by the desktop environment.

### On Windows
- Studio reuses the existing ProgID identifiers: `Xenapse.Diagram` and `Xenapse.Stylesheet`.
- Studio registers its launch command under the ProgIDs:
  ```reg
  [HKEY_CURRENT_USER\Software\Classes\Xenapse.Diagram\shell\open\command]
  @="\"C:\\Path\\To\\xenapse-studio.exe\" \"%1\""

  [HKEY_CURRENT_USER\Software\Classes\Xenapse.Stylesheet\shell\open\command]
  @="\"C:\\Path\\To\\xenapse-studio.exe\" \"%1\""
  ```
- Alternatively, Studio may register `XenapseStudio.Diagram` and add itself to `.xnp\OpenWithProgids`.
