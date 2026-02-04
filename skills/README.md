# Codex Skills (Repo Copy)

This directory contains the project Codex skills so contributors can install them
locally and keep them aligned with the workflow documented in `docs/static-recomp-skills.md`.

## Install
Copy one or more skills into your local Codex skills directory.

```bash
rsync -a skills/static-recomp-av-compare/ "$CODEX_HOME/skills/static-recomp-av-compare/"
```

## Upgrade
Re-run the same `rsync` command to update the local copy. If you have local edits,
back them up first to avoid overwriting.

## Notes
- Keep skill paths stable and avoid adding proprietary assets.
- Update `docs/static-recomp-skills.md` when the skill set changes.
