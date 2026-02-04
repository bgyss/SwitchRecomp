# DKCR HD First-Level Sample (Scaffolding)

This folder provides non-proprietary scaffolding for SPEC-200. It is not a playable build; it is a configuration and workflow template.

Included:
- `title.toml`: placeholder config with stub map and external asset/key paths.
- `provenance.toml`: placeholder provenance pointing at external inputs (XCI, keys, reference video).
- `module.json`: placeholder lifted module for pipeline wiring only.
- `patches/patches.toml`: placeholder patch set for first-level bring-up.

Notes:
- Update the external paths, sizes, and SHA-256 hashes in `provenance.toml` before running.
- Replace `module.json` with lifted output once XCI intake and lifting are wired up.
- All proprietary assets, keys, and videos remain external to the repo.

XCI intake (external tooling):
```
cargo run -p recomp-cli -- xci-intake \
  --xci /Volumes/External/DKCR_HD/game.xci \
  --keys /Volumes/External/SwitchKeys/prod.keys \
  --program-title-id 0100000000000000 \
  --provenance samples/dkcr-hd/provenance.toml \
  --out-dir out/dkcr-hd-intake \
  --xci-tool /usr/local/bin/hactool
```

Usage (from repo root):
```
cargo run -p recomp-cli -- run \
  --module samples/dkcr-hd/module.json \
  --config samples/dkcr-hd/title.toml \
  --provenance samples/dkcr-hd/provenance.toml \
  --out-dir out/dkcr-hd
```

Then build the emitted project:
```
cargo build --manifest-path out/dkcr-hd/Cargo.toml
```
