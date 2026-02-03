# XCI Intake Workflow (Scaffold)

This workflow ingests a user-supplied XCI and keyset, extracts ExeFS and NSO segments, and
emits RomFS assets into a separate output root. The current implementation includes a
mock extractor for non-proprietary tests and fixtures. Real-world extraction should be
wired to an external tool (e.g., hactool) in a private workspace.

## Inputs
- XCI image (path to `.xci`).
- Keyset (path to `.keys` or `.keyset`).
- Provenance metadata listing both inputs with hashes.

## CLI Usage
```bash
cargo run -p recomp-cli -- xci-intake \
  --xci path/to/title.xci \
  --keys path/to/title.keys \
  --provenance provenance.toml \
  --out-dir out/xci-intake \
  --assets-dir assets/xci-intake
```

Optional program selection:
```bash
cargo run -p recomp-cli -- xci-intake \
  --xci path/to/title.xci \
  --keys path/to/title.keys \
  --provenance provenance.toml \
  --config title.toml \
  --out-dir out/xci-intake \
  --assets-dir assets/xci-intake
```

The XCI intake config recognizes these optional fields at the top level:
- `program_title_id`
- `program_version`
- `program_content_type` (defaults to `program`)

## Provenance Requirements
The provenance file must list the XCI and keyset as inputs, for example:
```toml
[[inputs]]
path = "title.xci"
format = "xci"
sha256 = "..."
size = 123
role = "retail_image"

[[inputs]]
path = "title.keys"
format = "keyset"
sha256 = "..."
size = 456
role = "decryption_keys"
```

## Outputs
- `out_dir/exefs/` contains extracted ExeFS files.
- `out_dir/segments/` contains decompressed NSO segments.
- `out_dir/module.json` and `out_dir/manifest.json` record hashes and metadata.
- `assets_dir/romfs/` contains extracted RomFS assets.

## Mock Extractor
For tests and fixtures, the mock extractor expects a JSON payload in the `.xci` file:
```json
{
  "schema_version": "1",
  "programs": [
    {
      "title_id": "0100000000000000",
      "content_type": "program",
      "version": "1.0.0",
      "nca": { "data_b64": "..." },
      "exefs": [
        { "name": "main", "data_b64": "..." }
      ],
      "nso": [
        { "name": "main", "data_b64": "..." }
      ]
    }
  ],
  "romfs": { "image_b64": "..." }
}
```

## Notes
- The implementation refuses to place assets inside `out_dir` or vice versa.
- Real extraction should run outside the repo and only copy non-proprietary metadata
  into tracked files.
