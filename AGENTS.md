# Repository Guidelines

## Project Structure & Module Organization
- `specs/` holds the numbered specification series (see `specs/README.md` for ordering).
- `specs/SPEC-TEMPLATE.md` is the template for new specs.
- `ROADMAP.md` defines phases and exit criteria.
- `RESEARCH.md` tracks research directions and sources.
- `README.md` is the project overview and contribution entry point.

This repository is currently documentation‑only; no source code or tests exist yet.

## Build, Test, and Development Commands
There are no build or test commands at this stage. When tooling is introduced, add the canonical commands here with short descriptions (for example: `make build`, `ctest`, or `python -m pytest`).

## Coding Style & Naming Conventions
- Specs use the naming pattern `SPEC-XXX-SLUG.md` (e.g., `SPEC-030-RECOMP-PIPELINE.md`).
- Use concise headings, short paragraphs, and bullet lists for requirements.
- Keep text ASCII unless a source name requires Unicode.
- Prefer concrete, testable acceptance criteria.

## Testing Guidelines
No testing framework is defined yet. When tests exist, document:
- The framework (e.g., `pytest`, `ctest`).
- Test file naming patterns (e.g., `test_*.py`).
- How to run the full suite and targeted subsets.

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
