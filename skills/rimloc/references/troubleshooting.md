# Troubleshooting

| Symptom | Meaning | What to do |
|---|---|---|
| `rimloc.capabilities` tool itself absent | the rimloc MCP adapter has not shipped (Issue #10) | say exactly that; do not simulate tools and do not send the user hunting for an installer |
| `rimloc.capabilities` present but empty / connection refused | RimLoc not running or not listening | start RimLoc, retry once, then report what is absent |
| `rimloc.project.open` → "source mod root … is not a directory" | the folder is not a RimWorld mod | the folder must be a mod root (contains `About/About.xml`); ask the user for the right folder |
| paid provider 401 / invalid key | key wrong or expired in the OS keychain | the user fixes it in RimLoc's provider screen; never accept or paste keys in chat |
| local provider (Ollama) connection refused / model not found | Ollama not running or model not pulled | ask the user to start Ollama / pull the model; the local path is free, so retry after is fine |
| stale_epoch / stale_revision refusal | someone else wrote first | re-read `rimloc.project.snapshot`; re-confirm with the user; never retry blindly |
| source fingerprint refusal | mod changed since project creation | offer a rescan into a fresh project; never merge silently |
| draft present warning | GUI has unsaved work | do not apply over it; ask the user to save or discard first |
| export refusal | target inside source/managed tree | ask for another directory (guards are by design) |
