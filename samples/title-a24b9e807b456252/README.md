# title-a24b9e807b456252 First-Level Sample (Scaffolding)

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
  --xci /Volumes/External/title-a24b9e807b456252/xci/0123456789abcdef.xci \
  --keys /Volumes/External/SwitchKeys/prod.keys \
  --provenance samples/title-a24b9e807b456252/provenance.toml \
  --out-dir out/title-a24b9e807b456252-intake \
  --assets-dir out/title-a24b9e807b456252-assets \
  --xci-tool hactool \
  --xci-tool-path /usr/local/bin/hactool
```

Usage (from repo root):
```
cargo run -p recomp-cli -- run \
  --module samples/title-a24b9e807b456252/module.json \
  --config samples/title-a24b9e807b456252/title.toml \
  --provenance samples/title-a24b9e807b456252/provenance.toml \
  --out-dir out/title-a24b9e807b456252
```

Then build the emitted project:
```
cargo build --manifest-path out/title-a24b9e807b456252/Cargo.toml
```
