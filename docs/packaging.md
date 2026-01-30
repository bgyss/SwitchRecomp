# Packaging Layout and Policy

This document defines the bundle layout and asset separation rules for SwitchRecomp outputs.

## Bundle Layout (v1)
```
bundle/
  code/
    Cargo.toml
    src/
    manifest.json
  assets/
    README.txt
  metadata/
    provenance.toml
    bundle-manifest.json
```

## Asset Separation Rules
- `code/` contains generated source code and build metadata only.
- `assets/` is reserved for user-supplied data and must remain empty in distributions.
- `metadata/` contains provenance and checksum data required for auditing.

## Reference Packaging Command
```
cargo run -p recomp-cli -- package \
  --project-dir out/minimal \
  --provenance samples/minimal/provenance.toml \
  --out-dir out/bundle-minimal
```
