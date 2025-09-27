# Install RimLoc CLI

This page lists all supported installation methods and how to verify downloads.

## Option 1: Cargo (crates.io)

Fastest if you already have Rust installed.

```bash
cargo install rimloc-cli
```

Notes:
- Install Rust via https://rustup.rs if needed (Windows: run rustup‑init.exe).
- Ensure `~/.cargo/bin` is on PATH (open a new terminal after install). On Windows, restart PowerShell or add `%USERPROFILE%\.cargo\bin` to PATH.

## Option 2: GitHub Releases (binaries)

Releases page: https://github.com/0-danielviktorovich-0/RimLoc/releases

### Stable releases

Download the asset named `rimloc-cli-<tag>-<target>.<ext>` for your platform (choose the latest release that is NOT marked “Pre-release”):
- Linux (x86_64 GNU): `rimloc-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- Linux (x86_64 musl): `rimloc-cli-<tag>-x86_64-unknown-linux-musl.tar.gz`
- Linux (aarch64 GNU): `rimloc-cli-<tag>-aarch64-unknown-linux-gnu.tar.gz`
- Linux (aarch64 musl): `rimloc-cli-<tag>-aarch64-unknown-linux-musl.tar.gz`
- macOS (x86_64): `rimloc-cli-<tag>-x86_64-apple-darwin.tar.gz`
- macOS (arm64): `rimloc-cli-<tag>-aarch64-apple-darwin.tar.gz`
- Windows (x86_64): `rimloc-cli-<tag>-x86_64-pc-windows-msvc.zip`
- Windows (arm64): `rimloc-cli-<tag>-aarch64-pc-windows-msvc.zip`

### Dev pre-releases (nightly)

Download the asset named `rimloc-cli-dev-latest-<target>.<ext>` (attached to the latest dev pre-release):
- Linux (x86_64 GNU): `rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz`
- Linux (x86_64 musl): `rimloc-cli-dev-latest-x86_64-unknown-linux-musl.tar.gz`
- Linux (aarch64 GNU): `rimloc-cli-dev-latest-aarch64-unknown-linux-gnu.tar.gz`
- Linux (aarch64 musl): `rimloc-cli-dev-latest-aarch64-unknown-linux-musl.tar.gz`
- macOS (x86_64): use the tagged asset `rimloc-cli-<tag>-x86_64-apple-darwin.tar.gz` if no dev‑latest copy is present
- macOS (arm64): use the tagged asset `rimloc-cli-<tag>-aarch64-apple-darwin.tar.gz` if no dev‑latest copy is present
- Windows (x86_64): `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`
- Windows (arm64): `rimloc-cli-dev-latest-aarch64-pc-windows-msvc.zip`

Tip: if your platform has no `dev-latest` alias, use the matching tagged asset (same name without the `dev-latest` prefix).

## Verify Checksum

For each asset there is a `.sha256` file. Example on Linux/macOS:

```bash
cd ~/Downloads
sha256sum -c rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz.sha256
```

On Windows (PowerShell):

```powershell
Get-FileHash .\rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Get-Content .\rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip.sha256
```

## Verify Signature (optional)

Advanced users can verify signatures using `cosign` and the attached `.sig`/`.pem` files (available for Linux/macOS; Windows signature availability may depend on CI).

```bash
cosign verify-blob \
  --cert rimloc-cli-<tag>-<target>.<ext>.pem \
  --signature rimloc-cli-<tag>-<target>.<ext>.sig \
  rimloc-cli-<tag>-<target>.<ext>
```

## Unpack and Run

- Linux/macOS:

```bash
tar -xzf rimloc-cli-*.tar.gz -C /tmp
/tmp/rimloc-cli --help
```

- Windows:

```powershell
Expand-Archive -Path .\rimloc-cli-*.zip -DestinationPath .\rimloc
.\rimloc\rimloc-cli --help
```

For detailed step‑by‑step instructions on running the downloaded build on Windows/macOS/Linux (opening a terminal, common issues, etc.), see [Run Downloaded Build](install_run.md).

## Option 3: Build from source (all OS)

Use this if you prefer building from Git or want the very latest changes.

1) Install Rust toolchain (via rustup): https://rustup.rs
2) Clone the repo and build the CLI:

```bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
cargo build -p rimloc-cli --release
```

3) Run the built binary:

- Linux/macOS:

```bash
./target/release/rimloc-cli --version
# Optional: install to ~/.local/bin
install -Dm755 ./target/release/rimloc-cli ~/.local/bin/rimloc-cli
```

- Windows (PowerShell):

```powershell
.\u005ctarget\release\rimloc-cli.exe --version
# Optional: copy somewhere on PATH, e.g. %USERPROFILE%\bin
Copy-Item .\target\release\rimloc-cli.exe "$env:USERPROFILE\bin\rimloc-cli.exe"
```

## Fetch via gh (advanced)

If you have GitHub CLI installed and authenticated, you can fetch the latest dev pre-release programmatically:

```bash
REPO=0-danielviktorovich-0/RimLoc
TARGET=x86_64-unknown-linux-gnu   # change to your target triple
TAG=$(gh release list -R "$REPO" --limit 20 --json tagName,isPrerelease,createdAt \
  --jq '[.[] | select(.isPrerelease==true and (.tagName|test("-dev\\.")))] | sort_by(.createdAt) | last.tagName')
gh release download -R "$REPO" --tag "$TAG" --pattern "rimloc-cli-dev-latest-$TARGET.*" -D .
```
