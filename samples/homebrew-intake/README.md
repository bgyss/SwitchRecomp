# Homebrew Intake Sample

This walkthrough generates synthetic NRO/NSO inputs (plus a non-proprietary asset section) and runs the homebrew intake pipeline.

Usage (from repo root):

1) Generate inputs and provenance metadata.

```
python3 samples/homebrew-intake/generate.py
```

To skip the asset section, pass `--no-assets`.

2) Run homebrew intake.

```
cargo run -p recomp-cli -- homebrew-intake \
  --module samples/homebrew-intake/inputs/homebrew.nro \
  --nso samples/homebrew-intake/inputs/overlay.nso \
  --provenance samples/homebrew-intake/provenance.toml \
  --out-dir out/homebrew-intake
```

3) Inspect outputs.

```
ls out/homebrew-intake
```

The output includes:
- `segments/` with extracted NRO/NSO segments.
- `assets/` with `icon.bin`, `control.nacp`, and `romfs/romfs.bin` when assets are enabled.
- `module.json` and `manifest.json` describing the intake results.

All generated data is synthetic and non-proprietary.
