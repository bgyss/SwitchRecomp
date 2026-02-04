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
- Dev shell: `nix develop --impure`
- Run all tests: `cargo test`
- Run the sample pipeline:
  - `cargo run -p recomp-cli -- run --module samples/minimal/module.json --config samples/minimal/title.toml --provenance samples/minimal/provenance.toml --out-dir out/minimal`
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
- Always run the full suite after changes.
- Do not skip tests unless the user explicitly approves; record the approval and reason in the PR.

## Back Pressure Hooks
Pre-commit hooks provide fast feedback; `prek` is a drop-in replacement that reads the same `.pre-commit-config.yaml`.

- Install hooks: `prek install` or `pre-commit install`.
- Run on demand: `prek run --all-files` or `pre-commit run --all-files`.
- macOS note: the Nix dev shell ships `prek` only (to avoid Swift/.NET builds); install `pre-commit` separately if you need it.
- Configured hooks:
  - Pre-commit: `trailing-whitespace`, `end-of-file-fixer`, `check-merge-conflict`, `check-yaml`, `check-toml`, `check-json`, `check-added-large-files`, `detect-private-key`, `check-executables-have-shebangs`, `check-symlinks`, `check-case-conflict`, `cargo fmt --check`.
  - Pre-push: `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace`.
- If a pre-push hook fails, fix the reported changes, rerun the full test suite, then commit and push again after the hook clears.

## Commit & Pull Request Guidelines
No established commit conventions are present yet. Until standards are set:
- Use clear, imperative commit messages (e.g., "Add SPEC-070 OS services draft").
- PRs should include a short summary, a list of files touched, and any open questions.
- PRs must include a Testing section with the full suite (`cargo test`) and results.
- Do not use "Testing: Not run" unless the user explicitly approved skipping tests.
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
- Push all commits after creating them.

## Subagent Conflict Resolution
When using subagents, apply this workflow to keep ownership and diffs clear.

1) Record a clean baseline before spawning subagents: `git status -sb` and `git diff --stat`.
2) Assign each subagent a strict file or directory scope.
3) After each subagent finishes, compare changes to the baseline and declared scope.
4) If unexpected changes appear:
   - Stop all subagents.
   - Inspect the diff (`git diff --name-status` + `git diff` for unexpected files).
   - Decide to accept, revert, or move the changes into a separate commit.
5) Do not mix subagent outputs across scopes in a single commit.
6) If hooks modify files on commit/push, rerun the hook targets, re-stage, re-commit, then push again.
7) Only push when `git status -sb` shows a clean tree and hooks are green.
