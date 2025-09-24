---
title: Export and Import
---

# Export & Import

## Export PO

Export extracted strings into a unified `.po` file or separate `.po` files following the mod structure. Use `--help` to see all available options.

**Usage:**

```bash
rimloc-cli export-po [options]
```

**Options:**

| Option         | Description                                         | Required |
|----------------|-----------------------------------------------------|----------|
| `--out`        | Output file or directory for the exported `.po` files | No       |
| `--single-po`  | Export all strings into a single `.po` file          | No       |
| `--help`       | Show help information                                | No       |

**Examples:**

Dry run export:

```bash
rimloc-cli export-po
```
*Perform a dry run without writing any files.*

Export to a single PO file:

```bash
rimloc-cli export-po --out mymod.po --single-po
```
*Export all extracted strings into a single `.po` file named `mymod.po`.*

Export to multiple PO files following mod structure:

```bash
rimloc-cli export-po --out ./out
```
*Export strings into multiple `.po` files organized according to the mod structure, saved under `./out/`.*

**Tips:**

- Without `--out`, the command prints PO content to stdout—redirect it to capture the file.
- Combine with `--root` to target a specific mod when running outside the repository.
- Use `--dry-run` to preview which files would be created without writing to disk.

---

## Import PO

Import translations from a `.po` file back into XML files. This command supports a dry-run mode and can overwrite existing XML files or create new ones based on the imported translations.

**Usage:**

```bash
rimloc-cli import-po [options]
```

**Options:**

| Option      | Description                                  | Required |
|-------------|----------------------------------------------|----------|
| `--dry-run` | Perform a trial run without modifying files | No       |
| `--po`      | Path to the `.po` file to import             | No       |
| `--out`     | Output directory for generated XML files     | No       |
| `--help`    | Show help information                         | No       |

**Examples:**

Dry run import:

```bash
rimloc-cli import-po --dry-run
```
*Simulate the import process without changing any files.*

Import from a PO file and output to directory:

```bash
rimloc-cli import-po --po mymod.po --out ./out
```
*Import translations from `mymod.po` and write the resulting XML files to the `./out` directory.*

**Tips:**

- `--dry-run` is the safest way to inspect the changes before overwriting any XML.
- Specify `--mod-root` to point at the original mod if you need context for key lookups.
- Provide `--lang` when importing a language different from the PO file name.

---

## See also

- **[Scan XML](scan.md)** — Extract Keyed entries to work with.
- **[Validate](validate.md)** — Check duplicates, empty values, and placeholders in RimWorld mod XML files.
- **[Validate PO](validate_po.md)** — Check `.po` files for placeholder mismatches and strict mode issues.

## Troubleshooting

- **Missing keys on import** – run `scan` first and ensure the PO file was generated from the same mod structure.
- **Output directory left empty** – remember that `--out` points to the destination for generated XML; drop it to update files in place.
- **PO encoding issues** – PO files must be UTF-8; use `msgconv --output=utf-8` if a translator tool saved a different encoding.
