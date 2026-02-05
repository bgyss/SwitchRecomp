# Development

This project now includes an exploratory Rust workspace and uses Nix + devenv for the dev shell.

## Nix + devenv
- Install `nix` and `devenv`.
- This repo uses devenv's flake integration. Enter the shell from the repo root:

```
nix develop --impure
```

If you use direnv, run:
```
direnv allow
```

If `devenv.nix` changes, run `direnv reload` so nix-direnv refreshes the cached shell.

## Back Pressure Hooks (prek + pre-commit)
Back pressure keeps feedback close to the change. This repo uses pre-commit hooks for fast checks, and `prek` is a drop-in replacement that reads the same `.pre-commit-config.yaml`.

Install hooks (config sets `default_install_hook_types` to `pre-commit` + `pre-push`):
```
prek install
```
```
pre-commit install
```

Run hooks on demand:
```
prek run --all-files
```
```
pre-commit run --all-files
```

macOS note: the Nix dev shell ships `prek` only (to avoid Swift/.NET builds); install `pre-commit` separately if you need it.

Configured hooks:
- Pre-commit: `trailing-whitespace`, `end-of-file-fixer`, `check-merge-conflict`, `check-yaml`, `check-toml`, `check-json`, `check-added-large-files`, `detect-private-key`, `check-executables-have-shebangs`, `check-symlinks`, `check-case-conflict`, `cargo fmt --check`.
- Pre-push: `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace`.

## Testing Expectations
- Always run the full test suite (`cargo test`) after changes.
- PRs must include a Testing section that lists the command and outcome.
- Do not mark tests as "Not run" unless you have explicit approval to skip them.
- Include the test outcome in status updates so reviews never default to "Not run".

## Workspace Commands
- Run all tests:

```
cargo test
```

- Run baseline validation and emit reports:

```
cargo run -p recomp-validation -- --out-dir artifacts/validation
```

- Run validation from an artifact index (XCI intake + pipeline + captures):

```
cargo run -p recomp-validation -- artifacts --artifact-index /Volumes/External/validation/artifacts.json
```

- ISA unit tests live in `crates/recomp-isa` and validate arithmetic, shifts, load/store alignment, and flag updates.
- Service dispatch, timing trace, and graphics checksum tests live in their respective crates.

- Run the sample pipeline:

```
cargo run -p recomp-cli -- run \
  --module samples/minimal/module.json \
  --config samples/minimal/title.toml \
  --provenance samples/minimal/provenance.toml \
  --out-dir out/minimal
```

- Build the emitted project:

```
cargo build --manifest-path out/minimal/Cargo.toml
```

- Inspect the manifest:

```
cat out/minimal/manifest.json
```

- Package a bundle (code + metadata; assets supplied separately):

```
cargo run -p recomp-cli -- package \
  --project-dir out/minimal \
  --provenance samples/minimal/provenance.toml \
  --out-dir out/bundle-minimal
```

- Run homebrew intake (NRO + optional NSO inputs):

```
cargo run -p recomp-cli -- homebrew-intake \
  --module path/to/homebrew.nro \
  --nso path/to/optional.nso \
  --provenance path/to/provenance.toml \
  --out-dir out/homebrew-intake
```

- Lift homebrew intake output into a lifted module:

```
cargo run -p recomp-cli -- homebrew-lift \
  --module-json out/homebrew-intake/module.json \
  --out-dir out/homebrew-lift
```

The default mode attempts to decode a small AArch64 subset (mov wide, add, ret). Use `--mode stub`
to emit a placeholder lifted module without decoding instructions.
