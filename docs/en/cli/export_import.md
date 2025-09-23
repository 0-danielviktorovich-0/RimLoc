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

---

## See also

- **[Scan XML](scan.md)** — Extract Keyed entries to work with.
- **[Validate](validate.md)** — Check duplicates, empty values, and placeholders in RimWorld mod XML files.
- **[Validate PO](validate_po.md)** — Check `.po` files for placeholder mismatches and strict mode issues.
