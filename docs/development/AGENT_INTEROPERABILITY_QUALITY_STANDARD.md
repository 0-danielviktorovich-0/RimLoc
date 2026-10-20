# RimLoc Agent Interoperability & Quality Standard
Version: 1.0 (project policy) · Verified against public specifications: 2026-10-20 · Status: **binding for new agent/MCP/skill work** on the integration branch; does not certify existing code.

This is the **single source of truth for future agent-facing RimLoc features**. Read this before implementing Issue #10 (RimLoc external MCP), Issue #9 (RimWorld lab), any shipped skill, skill generator, integration, installer or change to a public agent API. Keep the public end-user documentation short; this document is the engineering contract. Use `AGENTS.md` as the entrypoint and link here; a new agent should not need the owner's historical chat.

## 0. Meaning, authority, change control

- Interpret uppercase **MUST / MUST NOT / SHOULD / MAY** as in IETF **BCP 14 (RFC 2119 + RFC 8174)**. These uppercase terms in this file state **RimLoc project requirements**, not claims that a referenced external specification requires the identical behavior.
- **Normative specification** = an explicitly identified release of a maintained protocol/format. **Security framework/checklist** = a source for engineering controls, not certification. **Project policy** = our additional enforceable product requirement. **Recommendation** = optional if genuinely inapplicable.
- Pin the **tested protocol revision**, preserve backward compatibility with older supported clients, and track a source URL, edition, verified date, applicability, implementation location, concrete test, owner and exceptions in a machine-readable compliance matrix. Check version changes when implementing; do not blindly chase drafts, unpublished proposals or every standard on every commit.
- Missing evidence means `NOT_VERIFIED`, not `PASS`. Never claim "ISO-certified", "OWASP-compliant", "SLSA Level N" or WCAG conformance from a single green CI badge. Exceptions require a documented rationale, risk, expiry/revisit trigger and owner.
- The existing product principle remains: **GUI-first**, local-first, GPL-3.0, canonical Rust service model, source-mod read-only, explicit guarded exports, durable locale-scoped drafts, version/epoch checks. MCP and skills augment it without duplicating translation logic. Owner-only gates for merging, tags, publication, paid-provider consent and community posts stay in force.

## 1. Authoritative standards and evidence requirements

| Domain | Reference / maturity | Project requirements / evidence |
| --- | --- | --- |
| Agent Skills packaging | **Agent Skills open specification** (format, metadata and progressive disclosure; de facto open format) | `SKILL.md` with valid YAML frontmatter `name` + `description`; skill directory name matches; descriptions say **when to use**; on-demand `references/` and `scripts/`; run `skills-ref validate`. |
| MCP API | **Model Context Protocol stable 2026-07-28**, plus documented compatibility to actually supported earlier revision(s) | Strict protocol version negotiation/detection, valid JSON-RPC, tools/resources/prompts as appropriate, errors and cancellation. **DO NOT implement the deprecated connection-only initialize model as the sole behavior on a current 2026-07-28 endpoint**. Test real clients for each advertised revision. |
| Skills through MCP | **Official `io.modelcontextprotocol/skills` extension, SEP-2640 Final**, stable Skills extension for MCP ≥2026-07-28 | Optional negotiated capability only. When enabled: resources + `skills/list`, `skills/get`, `resources/read`; complete file manifest with SHA-256 digests and byte sizes; safe origin/provenance; do not assume every client supports it. Provide ordinary filesystem skill distribution too. |
| Data contracts | **JSON Schema Draft 2020-12**; MCP-defined schema semantics take precedence | Typed input/output for each tool, strict validation, bounded sizes, predictable errors, contract tests, no manual shape guessing. Keep Rust/Tauri/client/MCP mappings synchronized. |
| Versioning | **Semantic Versioning 2.0.0** (with special pre-1.0 and prerelease caveats) | Document MCP/skill API stability and compatibility; test changes and migrations. SemVer does **not** mean users can guess whether game version is supported. |
| Secure software development | **NIST SP 800-218 SSDF v1.1** (public US guidance, not a project certification) | Threat model, code review, tests, secure defaults, security regression and vulnerability disclosure for external agent access. |
| Application, MCP & skill threats | **OWASP ASVS** (applicable controls); **OWASP MCP Top 10** and **OWASP Agentic Skills Top 10** (community guidance, evolving; verify current maturity) | Least privilege, isolation, no generic command execution, prompt/tool/skill-injection defenses, skill origin review, no token leakage, source mod protection, malicious project fixtures and negative tests. Do not claim standards certification. |
| Software supply chain | **SLSA v1.2**, **OpenSSF Best Practices/Scorecard**, **SPDX** (SBOM) as applicable | Pinned dependencies/actions, signed/attested release provenance where feasible, SBOM, vulnerability checks, artifact integrity and exact SHA. Adoption targets and gaps tracked, no fictional SLSA level. |
| Accessibility | **W3C WCAG 2.2 AA** for relevant Tauri WebView content; OS-native accessibility guidelines for native surfaces | Keyboard, focus, contrast, screen reader/semantic labels, zoom, localization, visible errors; automated tests **plus manual/user-flow audit**. WCAG is applied to applicable web UI, not asserted as automatic certification of the whole desktop app. |
| Runtime integration | Official Tauri 2 security/capability guidance + RimLoc canonical IPC service contract | Access limited by capabilities; external agent writes go through existing `ProjectSessionManager`, pinned revision + epoch + source fingerprint, atomic guarded apply; no direct source-tree write. |

Verified primary references (prefer these over third-party summaries):
- Agent Skills: https://agentskills.io/specification
- Agent Skills reference validator: https://github.com/agentskills/agentskills/tree/main/skills-ref
- MCP 2026-07-28: https://modelcontextprotocol.io/specification/2026-07-28
- MCP compatibility/transports: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports
- MCP Skills extension: https://modelcontextprotocol.io/extensions/skills/overview and https://skills.extensions.modelcontextprotocol.io/specification/stable/skills
- Official MCP Inspector: https://github.com/modelcontextprotocol/inspector
- Official MCP conformance: https://github.com/modelcontextprotocol/conformance
- JSON Schema: https://json-schema.org/specification
- SemVer: https://semver.org/
- BCP 14: https://www.rfc-editor.org/rfc/rfc2119 and https://www.rfc-editor.org/rfc/rfc8174
- NIST SSDF: https://csrc.nist.gov/pubs/sp/800/218/final
- OWASP ASVS: https://owasp.org/www-project-application-security-verification-standard/
- OWASP MCP Top 10: https://owasp.github.io/www-project-mcp-top-10/
- OWASP Agentic Skills Top 10: https://owasp.github.io/www-project-agentic-skills-top-10/
- SLSA: https://slsa.dev/spec/v1.2/
- OpenSSF: https://openssf.org/projects/best-practices-badge/ and https://scorecard.dev/
- SPDX: https://spdx.dev/specifications/
- WCAG: https://www.w3.org/TR/WCAG22/
- Tauri capabilities: https://v2.tauri.app/security/capabilities/

## 2. Public first-party Agent Skills: install once, use immediately

**Product requirement:** alongside the opt-in MCP server, distribute a **first-party, portable `rimloc` Agent Skill** that teaches any supported agent how to connect, discover actual capabilities, translate/review/validate/export safely and inspect a real result. A skill is instructions/workflow; **it does not replace the MCP server**, provide authority, magically install RimLoc or bypass tool permission checks.

**Suggested checked-in layout (agent to implement/test, not yet claimed shipped):**

~~~text
skills/
  rimloc/
    SKILL.md
    references/
      mcp-capabilities.md
      translate-review-export.md
      troubleshooting.md
      supported-hosts.md
    assets/
      sample-mod-metadata.json
  rimloc-verify-game/        # separate opt-in dev/test skill, not ordinary translator default
    SKILL.md
    references/ ...
  rimloc-skill-author/        # contributor scaffolding / validation tutorial
    SKILL.md
    references/ ...
skills/templates/             # starter template for community authors
~~~

Rules:
1. `skills/rimloc/SKILL.md` MUST satisfy official `name`, `description`, frontmatter and length conventions; the **trigger description** must distinguish translation from unrelated XML/game queries. Keep main skill compact (<500 lines recommended); move deep steps into references, loaded on demand.
2. First-party skill MUST discover live `rimloc.capabilities` and protocol revision, choose a supported tool path, verify project identity/source locale/target locale, check draft and revision before apply, and distinguish **AI proposal** from committed translation. No missing tool may be silently invented.
3. Provide one novice path, one professional path, examples in **English and Russian**, realistic sample mods that we own, error recovery, privacy/paid-provider notice and the negative path where RimLoc/server is unavailable.
4. **Installation UX:** direct install through compatible skills tooling (`npx skills add <owner/repo> -s rimloc` **only after verified in a real host**), native install instructions per tested Codex/Claude Code/ZCode/other hosts, and a manual/local/offline copy option. Disclose third-party installer dependencies, network access, telemetry/opt-out, exact source repo and version; no invisible auto-run scripts. Verify compatibility one host at a time, do not say "works with all agents" without test.
5. MCP may additionally **serve skills using the official `io.modelcontextprotocol/skills` extension** for clients that advertise support. This is an alternative **discovery/distribution path**, not a replacement for ordinary installable `SKILL.md` folders. Implement after standard MCP tools are stable and verify extension capability negotiation, listing/get/read, digests and identity.
6. Package skill files with proper provenance and license; do not embed game binaries, paid mod assets, live credentials, hidden ChatGPT session URLs, user data or remote executable setup commands.

## 3. Community skills and agent-assisted skill creation

**Outcome:** users AND their own agents can create new skills **for RimLoc**, share them, and validate them. This is distinct from the product's own built-in AI translation.

- Publish a first-party **skill authoring template** plus a `rimloc-skill-author` guide with valid SKILL.md, standard references/assets structure, examples and contribution instructions. A `rimloc skill init <name>` generator is **optional** and must use existing CLI/service conventions; avoid a UI wizard before need is evidenced. An agent can scaffold the template through its ordinary file tools and validator without any new GUI.
- New skill MUST document trigger/when-to-use, requirements, supported RimLoc/protocol versions, needed MCP capabilities, allowed side effects and recovery path, inputs/outputs, at least one positive & negative example, tests and author/license. Optional security metadata is **project convention**, not part of mandatory Agent Skills fields.
- Offer `skills-ref validate` and custom automated checks, example end-to-end trials against mock/fixture RimLoc MCP, and **real semantic client evaluations**. Verify that skill is triggered when it should, **does not activate** on unrelated prompts, does not invent tools, handles missing capabilities, invalid XML, stale revision, empty selection, no account/keys, and doesn't write source directories.
- External contributed skills MUST NOT get write permission, be installed automatically, or become "official" merely by existing in a repository or being read by an LLM. Review PR, license/security scan, pin versions, provenance, safe testing, and explicit user choice before install. Do not automatically publish user-created skills from the desktop app.
- A community skill may orchestrate **both** RimLoc MCP and RimWorld test bridge only with two explicitly separate scopes and owner-consented game-test profile. A translation skill MUST NOT silently boot games/download models/spend money.
- Document a community extension contract with a tested client/version matrix, examples and a clear support boundary. Aim for portable skills rather than tool-specific forks; platform-specific wrappers only when a real documented incompatibility requires one.

## 4. High-quality developer and user documentation

The feature is NOT finished without docs usable by a new user with no historical context:

- **Getting started (human):** what is a skill vs. MCP, system prerequisites, release/install, one-command setup if available, GUI and no-GUI paths, example "translate mod → review → export" with expected outcome, disable/uninstall/rollback and troubleshooting. No mandatory terminal for the ordinary RimLoc GUI product; shell instructions are for agents/power users.
- **Integrators:** contract revision, capability discovery, real tool schemas, permissions, transport/setup examples, live GUI synchronization and single writer, errors/retry/cancellation, invocation rate and payload limits, sample traces.
- **Skill developers:** template, standard spec, security model, tested host compatibility and how to submit skills.
- **Maintainers:** ADR for MCP/skills decisions, migration and deprecation policy, version support and risk register, reproducible test matrix. `AGENTS.md` should link here as policy entrypoint, not duplicate this long reference.
- Single source from **typed Rust contract** for generated tool schemas and capability tables when feasible, with drift detection. A doc page that says "planned" must not call the function "shipped" elsewhere. Every user-visible claim must map to an exact production artifact test and revision.

## 5. CI and release quality gates for skill/MCP work

Engineering lanes must provide auditable evidence:

- **Format:** `skills-ref validate`, YAML safe parser, file inventory/manifest, no dangling references, metadata validity, license/provenance check, RU/EN copy sanity.
- **Protocol:** official Inspector/Conformance tests for advertised MCP revision(s), real stdio handshake/discovery and typed calls from at least one **real supported client** (not only mocks); strict stdout = MCP messages, stderr = logs, invalid schema, version mismatch, cancel, resource bounds, no startup side effects, duplicate concurrent clients.
- **Skill evaluations:** 10+ representative task prompts over fixtures spanning positive/negative/non-trigger; at least 2 **real host** runs for claimed multi-host compatibility, plus a novice end-to-end "install → connect → one translation" path. Track success/failure, real model/tool trace when approved and token footprint; remove redundant context/tool enumeration.
- **App/integration:** real project create/search/propose/review/apply/validate/export; original sources SHA-256 unchanged; app running at same time with unsaved manual draft preserved; external edits reflected without stale overwrite. If in-game verifier skill is enabled, prove one live RimWorld label via Issue #9 lab with screenshot+semantic/game-state evidence and sandbox teardown.
- **Security:** source-path escape, malicious `SKILL.md` instructions, arbitrary command and credential scraping refusals, mismatched manifest digest/origin, prohibited remote listener, paid-provider starts without consent, stale revision/session; include adversarial tests as regression fixtures.
- **Supply chain:** fmt/clippy/tests, CodeQL, dependency advisories, exact commit, artifact SHA, dependency/skills lock/hashes and SBOM/attestation when ready. Record statuses as `PASS / FAIL / PARTIAL / NOT_TESTED`, not a guessed compliance score.

Evidence to deliver for each release candidate:
`FEATURE|STANDARD_REF|REVISION|CLAUSE_OR_CONTROL|TEST_ID|CLIENT/OS|SHA|RESULT|ARTIFACT|CAVEAT`.
Maintain a host matrix:
`HOST|VERSION|SKILL_INSTALL|MCP_STDIO|MCP_SKILLS_EXTENSION|GUI_SYNC|TEST_DATE|EVIDENCE`.
Publish *only* supported matrix rows, not optimistic compatibility claims.

## 6. Scope, ownership and sequencing

1. **Now:** existing safety P0/P1 and real-game acceptance from Issues #1/#9; minimal stdio MCP from Issue #10 in isolated worktree. AGENTS policy + skill spec/CI scaffold can progress in parallel without blocking first beta.
2. **Next:** validated installable `skills/rimloc` MVP, README quickstart and two compatible clients; independent test of end-user installation and safe real-mod translation. MCP Skills extension only where host support exists.
3. **Then:** live GUI/session broker + in-game automation skill + community authoring template/generator + compatible platforms, released on explicit owner gate.
4. **Never confuse:** a plan, reference source or SKILL.md file alone with a working MCP server/installed skill/live game UI. Maintain honest implementation status and prioritize an actual usable limited beta rather than waiting to become a universal automation platform.

## 7. Standards maintenance policy

At release preparation and on protocol/security-relevant changes, a designated lane checks the official sources above for newer stable revisions, current compatibility and any security advisories. It produces a short change-impact diff (required migration, recommended, irrelevant, deferred). The owner or designated maintainer approves breaking changes to public contracts; no silent schema rewrites or retroactive claims. Preserve a copy/reference of the precise version used in tests and link it from the evidence manifest.
