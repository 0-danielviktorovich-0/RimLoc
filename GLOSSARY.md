# RimLoc

Glossary for the RimLoc domain: a desktop-first RimWorld localization
studio (scan a mod, translate with AI or by hand, export a canonical
translation mod).

## Language

**RC bundle**:
A CI-staged, SHA256-hashed build of merged `main` for one platform that
awaits owner acceptance. The bundle identity (SHA + build SHA) lives in
`.rimloc-release-state.json`.
_Avoid_: artifact, build, installer (they name parts, not the staged set)

**Owner gate**:
A verdict meaning the mechanics are complete and verified, but the final
action or decision belongs to the owner and cannot be delegated.
_Avoid_: blocker (it is a verdict, not an obstacle)

**Owner UAT**:
The owner's acceptance test of RC bundles, performed by following
`UAT_GUIDE.md`; the only release stage agents cannot perform.
_Avoid_: QA, testing (those name agent-runnable lanes)

**Public beta**:
The first published build (GitHub pre-release) that users other than the
owner may download; not a launch, and Discord/Reddit announcements are a
separately gated step.
_Avoid_: release, launch

**Ad-hoc signing**:
A self-signed bundle seal without a paid Developer ID identity; passes
strict validation, but quarantined downloads show a documented
"open anyway" path instead of a clean install.
_Avoid_: unsigned (the bundle is signed; it just has no identity)

**Evidence run**:
A dated acceptance run whose result JSONs and a Player-log excerpt are
committed under `docs/development/evidence/YYYY-MM-DD/`; the full log stays
outside the repo.
_Avoid_: proof, test report
