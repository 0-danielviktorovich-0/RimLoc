# Install RimLoc CLI

This page shows where to download ready-to-use builds and how to verify them.

## Quick Download

- Releases page (pre-releases): https://github.com/0-danielviktorovich-0/RimLoc/releases
- Download the asset named `rimloc-cli-dev-latest-<target>.<ext>` for your platform:
  - Linux (x86_64 GNU): `rimloc-cli-dev-latest-x86_64-unknown-linux-gnu.tar.gz`
  - Linux (x86_64 musl): `rimloc-cli-dev-latest-x86_64-unknown-linux-musl.tar.gz`
  - Linux (aarch64 GNU): `rimloc-cli-dev-latest-aarch64-unknown-linux-gnu.tar.gz`
  - Linux (aarch64 musl): `rimloc-cli-dev-latest-aarch64-unknown-linux-musl.tar.gz`
  - macOS (x86_64): `rimloc-cli-dev-latest-x86_64-apple-darwin.tar.gz` (no latest copy; use tagged asset)
  - macOS (arm64): `rimloc-cli-dev-latest-aarch64-apple-darwin.tar.gz` (no latest copy; use tagged asset)
  - Windows (x86_64): `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`
  - Windows (arm64): `rimloc-cli-dev-latest-aarch64-pc-windows-msvc.zip`

Note: dev-latest files are attached to the latest dev pre-release. If your platform has no `dev-latest` copy, use the matching tagged asset (same name without `dev-latest` prefix).

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
.\rimloc\rimloc-cli.exe --help
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

