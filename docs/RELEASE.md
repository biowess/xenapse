# Xenapse Release Guide (v0.1.0)

This guide documents the procedures for packaging, verifying, and distributing Xenapse release assets.

> **Status Notice**: Configuration and metadata have been verified statically via `cargo metadata`. The build and packaging procedures described below require manual execution on the designated host environments. Release artifacts have not been pre-built or uploaded.

---

## 1. Required Build Environments

| Platform | Target Architecture | Environment | Toolchain / Prerequisites |
|---|---|---|---|
| **Linux** | `x86_64-unknown-linux-gnu` | Ubuntu 22.04 LTS (Docker container) | Stable Rust (edition 2021), `build-essential`, `tar`, `coreutils` |
| **Windows** | `x86_64-pc-windows-msvc` | Windows 10 VM | Stable Rust (edition 2021, MSVC), PowerShell 5.1+ |
| **macOS** | Source-build support | macOS (x86_64 / arm64) | Stable Rust (edition 2021), Cargo |

---

## 2. Archive Naming Conventions

Release archives must strictly follow these names:

- **Linux x86_64**: `xenapse-0.1.0-linux-x86_64.tar.gz`
  - Checksum: `xenapse-0.1.0-linux-x86_64.tar.gz.sha256`
- **Windows x86_64**: `xenapse-0.1.0-windows-x86_64.zip`
  - Checksum: `xenapse-0.1.0-windows-x86_64.zip.sha256`

### Archive Contents
Each archive contains:
- Executable binary (`xenapse` on Linux, `xenapse.exe` on Windows)
- `LICENSE` (Apache License 2.0 text)
- `NOTICE` (Attribution notice)
- `README.md` (Project overview and CLI usage)
- `packaging/` (OS-specific file type definitions, icons, and registration/uninstallation scripts)

---

## 3. Linux Native Build (Ubuntu 22.04 Docker)

Execute the build inside an Ubuntu 22.04 container to establish a glibc 2.35 baseline. The binary is built in an Ubuntu 22.04 environment using glibc 2.35. This establishes the intended Linux compatibility baseline; compatibility with other distributions has not been independently tested.

### Procedure

1. Open a terminal at the repository root.
2. Run the packaging build in the Ubuntu 22.04 Docker container:

```bash
docker run --rm -v "$(pwd)":/workspace -w /workspace ubuntu:22.04 bash -c "
  set -euo pipefail
  apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential ca-certificates tar
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  source \$HOME/.cargo/env
  bash scripts/package-linux.sh
"
```

3. The generated release artifacts will be placed in the `dist/` directory:
   - `dist/xenapse-0.1.0-linux-x86_64.tar.gz`
   - `dist/xenapse-0.1.0-linux-x86_64.tar.gz.sha256`

---

## 4. Windows Native Build (Windows 10 VM)

Execute the build natively inside the Windows 10 VM using PowerShell.

### Procedure

1. Launch PowerShell in the cloned repository root inside the Windows 10 VM.
2. Ensure the Rust toolchain is available in the path (`rustc --version`, `cargo --version`).
3. Run the Windows packaging script:

```powershell
.\scripts\package-windows.ps1
```

4. The generated release artifacts will be placed in the `dist\` directory:
   - `dist\xenapse-0.1.0-windows-x86_64.zip`
   - `dist\xenapse-0.1.0-windows-x86_64.zip.sha256`

---

## 5. macOS Source-Build Support & Installation

Xenapse does not distribute pre-built macOS binaries; macOS is provided as source-build support instead. macOS users build and install directly from source via Cargo.

### Instructions for macOS Users

1. **Obtain the source repository**:
   ```bash
   git clone https://github.com/biowess/xenapse.git
   cd xenapse
   ```

2. **Install Rust prerequisites** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```
   *Requires a stable Rust toolchain supporting edition 2021.*

3. **Build the project**:
   ```bash
   cargo build --release -p xenapse-cli
   ```
   The compiled binary will be located at `target/release/xenapse`.

4. **Install the CLI locally**:
   ```bash
   cargo install --path crates/xenapse-cli
   ```
   This installs `xenapse` into `$HOME/.cargo/bin`. Ensure `$HOME/.cargo/bin` is in your `PATH`.

5. **Verify installation**:
   ```bash
   xenapse --version
   xenapse --help
   ```

---

## 6. SHA-256 Verification

Before uploading or after downloading release assets, verify the SHA-256 checksums:

- **Linux**:
  ```bash
  sha256sum -c xenapse-0.1.0-linux-x86_64.tar.gz.sha256
  ```

- **macOS**:
  ```bash
  shasum -a 256 xenapse-0.1.0-linux-x86_64.tar.gz
  # Compare with contents of xenapse-0.1.0-linux-x86_64.tar.gz.sha256
  ```

- **Windows (PowerShell)**:
  ```powershell
  Get-FileHash -Algorithm SHA256 .\xenapse-0.1.0-windows-x86_64.zip
  # Compare with contents of xenapse-0.1.0-windows-x86_64.zip.sha256
  ```

---

## 7. Manual GitHub Release Preparation

No automated CI/CD workflows are used. Prepare the release manually:

1. **Tag the release**:
   ```bash
   git tag -a v0.1.0 -m "Xenapse 0.1.0"
   git push origin v0.1.0
   ```
2. **Draft GitHub Release**:
   - Navigate to GitHub repository Releases -> "Draft a new release".
   - Select tag: `v0.1.0`.
   - Release title: `Xenapse 0.1.0`.
   - Description: Include release highlights, changelog, and SHA-256 checksum table.
3. **Upload release assets**:
   - `xenapse-0.1.0-linux-x86_64.tar.gz`
   - `xenapse-0.1.0-linux-x86_64.tar.gz.sha256`
   - `xenapse-0.1.0-windows-x86_64.zip`
   - `xenapse-0.1.0-windows-x86_64.zip.sha256`
4. **Publish release**.

---

## 8. Version Bumping for Future Releases

When bumping versions for subsequent releases:

1. Update `version` in the root `Cargo.toml` under `[workspace.package]`:
   ```toml
   [workspace.package]
   version = "0.2.0"
   ```
2. Update the `xenapse-core` version requirement in `crates/xenapse-cli/Cargo.toml`:
   ```toml
   [dependencies]
   xenapse-core = { path = "../xenapse-core", version = "0.2.0" }
   ```
3. Update `VERSION` in `scripts/package-linux.sh` and `scripts/package-windows.ps1`.
4. Refresh `Cargo.lock`:
   ```bash
   cargo check
   ```
