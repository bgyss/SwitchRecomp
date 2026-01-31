# Release Checklist

## Legal and Policy
- Confirm `docs/LEGAL-POLICY.md` is up to date.
- Verify no proprietary assets, keys, or binaries are included in the release.
- Confirm provenance metadata is present for all sample inputs.

## Build and Packaging
- `cargo test` passes.
- Sample pipeline completes without warnings.
- Bundle layout matches `docs/packaging.md`.
- `bundle-manifest.json` checksums match bundle contents.

## Documentation
- Update `README.md` usage instructions if CLI changed.
- Record spec status updates for completed work.
- Note any open questions or limitations.
