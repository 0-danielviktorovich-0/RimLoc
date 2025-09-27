---
title: Run Downloaded Build
---

# Run Downloaded Build

RimLoc CLI is a console application. If you double‑click the file, a window may flash and close immediately. Run it from a terminal instead, then pass commands and paths.

## What is a terminal?

- Windows: PowerShell or Windows Terminal
- macOS: Terminal app
- Linux: Terminal (e.g., GNOME Terminal, Konsole)

Open a terminal in the folder where you unpacked `rimloc-cli` and run commands shown below.

## Windows (PowerShell)

1) Unpack the ZIP you downloaded (e.g., `rimloc-cli-dev-latest-x86_64-pc-windows-msvc.zip`). You will get `rimloc-cli` (File Explorer may hide the `.exe` extension).
2) Open PowerShell in that folder:
   - File Explorer → navigate to the folder → type `powershell` in the address bar → Enter; or
   - Right‑click in the folder background → “Open in Terminal”.
3) Run the CLI:

```powershell
.\rimloc-cli --help
.\rimloc-cli --version
```

Important: PowerShell does not search the current folder by default. Run it as `\.\rimloc-cli` (or `\.\rimloc-cli.exe`) from the unpacked folder, or add the folder to PATH. Typing just `rimloc-cli` will fail with “not recognized” unless it is on PATH.

4) Basic usage examples (put your mod folder next to the EXE or use an absolute path):

```powershell
# List translation units (text output)
.\rimloc-cli scan --root .\MyMod --format text

# Validate XML translations
.\rimloc-cli validate --root .\MyMod

# Export a single PO file
.\rimloc-cli export-po --root .\MyMod --out-po .\MyMod.ru.po --lang ru

# Preview building a translation-only mod from PO
.\rimloc-cli build-mod --po .\MyMod.ru.po --out-mod .\MyMod_RU --lang ru --dry-run
```

Tip: Run from PowerShell, not by double‑clicking, so you can see output and errors.

Add to PATH (optional): create `%USERPROFILE%\bin`, copy `rimloc-cli.exe` (shown as `rimloc-cli` in Explorer) there, and add that folder to System → Environment Variables → Path.

Notes for PowerShell users:
- Absolute paths with spaces must be quoted, e.g. `--root "C:\\Games\\RimWorld Mods\\MyMod"`.
- If you redirect output to a file and need UTF‑8, prefer `| Out-File -Encoding utf8 file.json` instead of `> file.json` in Windows PowerShell 5. In PowerShell 7+, `>` writes UTF‑8 by default.

## macOS (Terminal)

1) Unpack the `tar.gz` archive (Finder or terminal):

```bash
tar -xzf rimloc-cli-*.tar.gz -C "$HOME/Downloads/rimloc"
cd "$HOME/Downloads/rimloc"
```

2) Make sure it’s executable and clear quarantine if needed:

```bash
chmod +x ./rimloc-cli
# If macOS shows a security prompt or “cannot be opened”:
xattr -d com.apple.quarantine ./rimloc-cli 2>/dev/null || true
```

3) Run the CLI:

```bash
./rimloc-cli --help
./rimloc-cli --version
```

4) Basic usage:

```bash
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
./rimloc-cli export-po --root ./MyMod --out-po ./MyMod.ru.po --lang ru
./rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
```

Add to PATH (optional): `install -Dm755 ./rimloc-cli ~/.local/bin/rimloc-cli` and ensure `~/.local/bin` is on PATH, or copy to `/usr/local/bin` (may require `sudo`).

## Linux

1) Unpack and enter the folder:

```bash
tar -xzf rimloc-cli-*.tar.gz -C "$HOME/Downloads/rimloc"
cd "$HOME/Downloads/rimloc"
chmod +x ./rimloc-cli
```

2) Run the CLI:

```bash
./rimloc-cli --help
./rimloc-cli --version
```

3) Basic usage:

```bash
./rimloc-cli scan --root ./MyMod --format text
./rimloc-cli validate --root ./MyMod
./rimloc-cli export-po --root ./MyMod --out-po ./MyMod.ru.po --lang ru
./rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
```

If you see “No such file or directory” on older distros, try the `-musl` build instead of `-gnu`.

Add to PATH (optional): `install -Dm755 ./rimloc-cli ~/.local/bin/rimloc-cli`.

## Common issues

- Window opens and closes: run from a terminal rather than double‑clicking.
- “command not found” or “not recognized”: use `./rimloc-cli` (Linux/macOS) or `.\rimloc-cli` (Windows) from the current folder, or add to PATH.
- Permission denied (Linux/macOS): `chmod +x ./rimloc-cli`.
- macOS security prompt: allow under System Settings → Privacy & Security → “Open Anyway”, or clear quarantine via `xattr -d com.apple.quarantine ./rimloc-cli`.
- Wrong architecture: download the asset matching your CPU and OS (e.g., `aarch64-apple-darwin` for Apple Silicon, `x86_64-apple-darwin` for Intel Macs).
- Old Linux glibc: prefer the `-musl` build.

## Next steps

- See the CLI overview for commands and flags: CLI → Overview.
- Learn specific tasks: Scan, Validate, Export/Import, Build Mod.
- Prefer easy updates? Use `cargo install rimloc-cli` from crates.io.
