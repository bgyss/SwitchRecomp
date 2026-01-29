# Minimal Sample

This folder holds a tiny JSON module and TOML config to exercise the exploratory pipeline.

Usage (from repo root):
- Build the CLI and run the pipeline.

```
cargo run -p recomp-cli -- \
  --module samples/minimal/module.json \
  --config samples/minimal/title.toml \
  --out-dir out/minimal
```

Then build the emitted project:

```
cargo build --manifest-path out/minimal/Cargo.toml
```

The output directory also includes `manifest.json` with hashes for the input module/config.
