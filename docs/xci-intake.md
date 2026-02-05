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
  --assets-dir assets/xci-intake \
  --xci-tool auto
```

Optional program selection:
```bash
cargo run -p recomp-cli -- xci-intake \
  --xci path/to/title.xci \
  --keys path/to/title.keys \
  --provenance provenance.toml \
  --config title.toml \
  --out-dir out/xci-intake \
  --assets-dir assets/xci-intake \
  --xci-tool hactool
```

Tool selection:
- `--xci-tool auto` (default): use `hactoolnet` or `hactool` if found on `PATH`, else fall back to the mock extractor.
- `--xci-tool hactool` or `--xci-tool hactoolnet`: require the specified tool.
- `--xci-tool mock`: force the mock extractor even if tools are available.
- `--xci-tool-path /path/to/hactool`: override the tool executable location.

Environment overrides:
- `RECOMP_XCI_TOOL=auto|hactool|hactoolnet|mock`
- `RECOMP_XCI_TOOL_PATH=/path/to/hactool`

The XCI intake config recognizes these optional fields at the top level:
- `program_title_id`
- `program_version`
- `program_content_type` (defaults to `program`)

## Real XCI Intake How-To
Use this flow when you have lawful access to a retail XCI and keyset. Keep inputs and
extracted assets outside the repo, and copy only non-proprietary metadata into tracked
files.

1. Prepare a private workspace with separate roots for inputs, intake metadata, and assets.
2. Place the `.xci` and keyset in the inputs root.
3. Hash the inputs and update your provenance file with `path`, `sha256`, and `size`.
4. Ensure `hactool` or `hactoolnet` is installed and set `--xci-tool` (plus
   `--xci-tool-path` if needed).
5. Run intake with absolute paths and separate output roots.
6. Validate the intake manifest before using the outputs downstream.
7. Copy only `module.json` and `manifest.json` into the repo (keep `assets_dir` external).
8. Record tool versions and command lines in provenance notes.

Example (external workspace):
```bash
cargo run -p recomp-cli -- xci-intake \
  --xci /Volumes/Inputs/title.xci \
  --keys /Volumes/Keys/prod.keys \
  --provenance /Volumes/Inputs/provenance.toml \
  --out-dir /Volumes/Outputs/title-intake \
  --assets-dir /Volumes/Outputs/title-assets \
  --xci-tool hactool \
  --xci-tool-path /usr/local/bin/hactool

recomp-cli xci-validate --manifest /Volumes/Outputs/title-intake/manifest.json
```

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
- External extraction uses `--outdir`, `--exefsdir`, and `--romfsdir` flags that are
  compatible with recent `hactool`/`hactoolnet` builds; adjust tool paths if needed.
