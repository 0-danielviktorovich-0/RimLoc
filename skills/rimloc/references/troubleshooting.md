# Troubleshooting

| Symptom | Meaning | What to do |
|---|---|---|
| `rimloc.capabilities` missing/empty | adapter or app not connected | start RimLoc, check the MCP adapter; report exactly what's absent |
| stale_epoch / stale_revision refusal | someone else wrote first | re-read `rimloc.project.snapshot`; re-confirm with the user; never retry blindly |
| source fingerprint refusal | mod changed since project creation | offer a rescan into a fresh project; never merge silently |
| draft present warning | GUI has unsaved work | do not apply over it; ask the user to save or discard first |
| export refusal | target inside source/managed tree | ask for another directory (guards are by design) |
