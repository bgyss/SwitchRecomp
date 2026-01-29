# Development

This project now includes an exploratory Rust workspace and uses Nix + devenv for the dev shell.

## Nix + devenv
- Install `nix` and `devenv`.
- Enter the shell from the repo root:

```
devenv shell
```

If you use direnv, run:
```
direnv allow
```

## Workspace Commands
- Run all tests:

```
cargo test
```

- ISA unit tests live in `crates/recomp-isa` and validate arithmetic + flag updates.

- Run the sample pipeline:

```
cargo run -p recomp-cli -- \
  --module samples/minimal/module.json \
  --config samples/minimal/title.toml \
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
