# Repository Guidelines

## Project Overview
SwitchRecomp is a preservation-focused static recompilation research project. The repository now includes an exploratory Rust workspace that mirrors the intended pipeline shape (config-driven recompilation and a minimal runtime ABI) while keeping inputs and outputs cleanly separated.

## Project Structure & Module Organization
- `specs/` holds the numbered specification series (see `specs/README.md` for ordering).
- `specs/SPEC-TEMPLATE.md` is the template for new specs.
- `ROADMAP.md` defines phases and exit criteria.
- `RESEARCH.md` tracks research directions and sources.
- `crates/` contains the exploratory Rust pipeline/runtime scaffolding.
- `samples/` contains small, non-proprietary inputs to exercise the pipeline.
- `docs/` holds development and design notes.
- `README.md` is the project overview and contribution entry point.

## Build, Test, and Development Commands
- Dev shell: `devenv shell`
- Run all tests: `cargo test`
- Run the sample pipeline:
  - `cargo run -p recomp-cli -- --module samples/minimal/module.json --config samples/minimal/title.toml --out-dir out/minimal`
- Build emitted output:
  - `cargo build --manifest-path out/minimal/Cargo.toml`

## Coding Style & Naming Conventions
- Specs use the naming pattern `SPEC-XXX-SLUG.md` (e.g., `SPEC-030-RECOMP-PIPELINE.md`).
- Use concise headings, short paragraphs, and bullet lists for requirements.
- Keep text ASCII unless a source name requires Unicode.
- Prefer concrete, testable acceptance criteria.

## Testing Guidelines
- Use `cargo test` for Rust workspace tests.
- Test files live under `crates/*/tests/` or inline in modules.
- Always run the full suite after changes unless the user explicitly says not to.

## Commit & Pull Request Guidelines
No established commit conventions are present yet. Until standards are set:
- Use clear, imperative commit messages (e.g., "Add SPEC-070 OS services draft").
- PRs should include a short summary, a list of files touched, and any open questions.
- Link related issues if available.

## Research & Sources
- Add new sources to `RESEARCH.md` with a short note on relevance.
- Prefer primary technical references; avoid linking to proprietary assets.
- Keep asset separation explicit in all specs and docs.

## Agent-Specific Instructions
- Keep edits small and focused per change.
- Update the appropriate spec status when making substantive changes.
- Do not add proprietary binaries, keys, or assets to the repository.
- Always test changes, update relevant documentation, and commit all code you modify or add.
