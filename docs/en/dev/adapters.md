---
title: Localization Adapters
---

# Adding support for another game or application

RimLoc is RimWorld-first, but the generic localization core is designed around adapters.

The goal is that a developer — or a coding agent — can add a new environment without rewriting the editor, TM, glossary, validation framework, project history, or target-locale model.

## What exists today

Two adapter identities already exercise the seam conceptually:

- RimWorld — the primary production target;
- RimLoc application/self-localization — a second, structurally different source.

This proves that the core project model is not supposed to equal “RimWorld XML”.

External third-party adapter ABI/marketplace is **not stable yet**.

## Core vs adapter

Generic core owns concepts such as:

- Project;
- SourceEntry;
- Translation;
- Locale;
- revision/history;
- TM;
- glossary;
- review state;
- findings;
- provenance;
- output/build artifacts.

An adapter owns environment-specific behavior such as:

- discovery/detection;
- source parsing;
- effective-content/version resolution;
- mapping into canonical SourceEntry records;
- importing existing translations;
- adapter-specific validation;
- build/export;
- optional runtime verification.

Do not add ThingDef, Minecraft keys, Steam Workshop rules, or another platform’s object model to the generic core.

## Minimal authoring checklist

When implementing an in-tree adapter:

1. Choose a stable adapter ID and schema/API version.
2. Define the source kinds it supports.
3. Implement source detection/discovery.
4. Produce deterministic canonical entries with stable IDs.
5. Preserve source location/provenance.
6. Implement existing-translation import if meaningful.
7. Implement update semantics (unchanged/new/source-changed/obsolete/ambiguous where applicable).
8. Declare capabilities.
9. Implement safe build/export.
10. Add fixtures and conformance tests.

## Capability-driven UI

The UI should ask the adapter what it supports rather than hard-code one universal workflow.

Examples:

- supports existing translation import;
- supports build/export;
- supports source context;
- supports dependencies;
- supports versions/content roots;
- supports runtime validation.

Unsupported capabilities should not be rendered as working controls.

## Conformance expectations

A useful adapter test suite should verify at least:

- deterministic inventory;
- stable IDs;
- no writes to source directories;
- target-locale isolation;
- restart/project persistence;
- update behavior;
- round-trip/build correctness where applicable;
- path containment;
- clean refusal for unsupported/unknown semantics.

## Simple formats vs complex platforms

Long-term RimLoc may support multiple adapter classes:

1. **Declarative/simple adapters** for JSON/CSV/gettext-like resources.
2. **Built-in Rust adapters** for complex game semantics.
3. **External versioned/sandboxed adapters** if ecosystem demand justifies a plugin protocol.

Do not use a Rust dynamic-library ABI as the public long-term plugin contract merely because it is easy to prototype. A future external boundary should be versioned and language-neutral (for example a process/RPC or WASM-based contract) after proper design.

## Good first external proof

A simple JSON-based application/game localization adapter is a better proof of extensibility than prematurely implementing five game ecosystems.

The first public beta does **not** require Minecraft/Paradox/etc. support. It requires the architecture to make those additions bounded later.

## Human/AI contributor workflow

When asking a coding agent to add an adapter, give it:

- this page;
- [the canonical architecture document](../../architecture/LOCALIZATION_ADAPTERS.md);
- a nearby reference adapter;
- fixtures;
- the conformance tests;
- explicit read-only source/output rules.

The desired result is a focused adapter implementation, not changes spread across every RimLoc layer.
