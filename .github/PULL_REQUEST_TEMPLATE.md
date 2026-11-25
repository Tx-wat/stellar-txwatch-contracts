## Summary

<!-- Describe the change and the motivation behind it. -->

## Contract Changes

- [ ] Unit tests added/updated for changed functions
- [ ] Integration tests added if cross-contract behavior changed
- [ ] CHANGELOG.md updated under [Unreleased]
- [ ] docs/alert-registry.md or docs/watcher-registry.md updated
- [ ] docs/events.md updated if events were added/changed
- [ ] TypeScript bindings regenerated if function signatures changed
- [ ] No new clippy warnings introduced
- [ ] Auth requirements verified (`require_auth()` present on all mutating functions)
- [ ] `ContractError` variants documented in rustdoc
- [ ] Updated WASM size table in README if contract binary size changed by more than 5 KB

## Checklist
- [ ] My code follows the repository style
- [ ] I have added relevant tests (unit / integration)
- [ ] I have updated or added documentation if necessary
- [ ] I have updated `CHANGELOG.md` under `[Unreleased]` (required for contract behavior changes, new functions, and bug fixes; optional but encouraged for docs/tests/CI)
- [ ] All CI checks pass

- [ ] `cargo test` passes locally
- [ ] Relevant docs in `docs/` updated
- [ ] `DEPLOYMENTS.md` updated if contract addresses changed

## Release notes / changelog
- Short note for changelog if this should be documented for release.
- Example entry format (see the "Changelog Updates" section in `CONTRIBUTING.md`):
  ```
  ### Added
  - New `transfer_with_memo` entry point (#123)
  ```

<!-- e.g. Closes #123 -->
