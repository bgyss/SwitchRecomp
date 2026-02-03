# Memory Image Sample

This sample demonstrates the memory image initialization flow by pairing a lifted
`module.json` with an initial data segment blob.

## Files
- `module.json`: Declares a data segment that is initialized from `data.bin` and
  zero-fills the remaining bytes.
- `data.bin`: Four bytes of initial data (0x01 0x02 0x03 0x04).
- `title.toml`: Configures a memory layout that maps the data segment region.
- `provenance.toml`: Minimal provenance metadata for the sample module.

## How To Run
From the repo root:

```bash
cargo run -p recomp-cli -- run \
  --module samples/memory-image/module.json \
  --config samples/memory-image/title.toml \
  --provenance samples/memory-image/provenance.toml \
  --out-dir out/memory-image
```

The emitted `manifest.json` will include a `memory_image` section describing the
segment blob, and the generated `main.rs` will apply the memory image before
calling `entry`.
