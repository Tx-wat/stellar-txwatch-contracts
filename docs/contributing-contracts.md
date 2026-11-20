# Contributing Contracts

This guide covers how to contribute to the Soroban contracts in this repository.

## Getting Started

1. Fork the repository and create a feature branch.
2. Install the Rust toolchain and `cargo`.
3. Run the existing test suite before making changes.

## General Contract Contributions

- Keep changes focused and minimal.
- Match existing module structure, naming, and error handling.
- Add or update tests for any behavior change.
- Update documentation when the public surface changes.

## Adding a New Rule Descriptor

Rule descriptors (for example `rule:invoke`, `rule:burn`) are the string tags that
contracts and the off-chain watcher use to identify a rule's semantics. Adding a
new descriptor touches several layers, so follow the full process below.

1. **Propose** — open an issue describing the descriptor name, its semantics, and
the use case it enables. Get agreement on the name and meaning before writing code.
2. **Contract change** — in `contracts/alert-registry/src/lib.rs`, add the descriptor
to the allow-list in `validate_rule`. That function is the single place that decides
whether a `rules` entry is accepted; `validate_rules` calls it for every entry and
also rejects empty lists, lists over 50 entries, and duplicates. There is no
`get_supported_rules` function: do not look for one.
   - If the descriptor needs a shared constant, define it next to the other rule
     strings in the same file so `validate_rule` and the tests use the same value.
3. **Watcher change** — implement the matching logic for the descriptor in
`stellar-txwatch-core` so the off-chain watcher can act on it.
4. **Docs** — update the rule descriptor table in `docs/alert-registry.md` to
document the new descriptor, and add a `CHANGELOG.md` entry under `[Unreleased]`.
5. **Tests** — in `contracts/alert-registry/src/`:
   - Add a positive case: a `register_alert` (or `update_alert`) call that uses the
     new descriptor succeeds and the stored config contains it.
   - Add a negative case: a near-miss spelling is rejected with
     `ContractError::InvalidRuleDescriptor`.
   - Add a duplicate case: the descriptor listed twice is rejected with
     `ContractError::DuplicateRule`.
   - Add an integration test in the `stellar-txwatch-core` crate covering the watcher logic.
   - Run `cargo test` and `cargo fmt --all -- --check` before opening the PR.
6. **TypeScript** — regenerate the TypeScript bindings if the type surface changes.

### Checklist for a new descriptor

| Where | What to change |
|---|---|
| `contracts/alert-registry/src/lib.rs` (`validate_rule`) | Add the descriptor string to the allow-list |
| `contracts/alert-registry/src/lib.rs` tests | Positive, near-miss, and duplicate cases (see step 5) |
| `docs/alert-registry.md` | Add a row to "Valid rule prefixes" |
| `CHANGELOG.md` | Entry under `[Unreleased]` → **Added**, referencing the issue |
| `stellar-txwatch-core` | Matching logic and integration test |
| Bindings (if the type surface changes) | Regenerate TypeScript bindings |

### Example PR Template for New Rule Descriptors

When opening a PR that adds a new rule descriptor, include the following checklist
in the PR description:

```markdown
## New Rule Descriptor: `rule:<name>`

- [ ] Proposal issue linked (name, semantics, use case)
- [ ] Descriptor added to the `validate_rule` allow-list in `contracts/alert-registry/src/lib.rs`
- [ ] Positive, near-miss, and duplicate tests added
- [ ] `CHANGELOG.md` entry added under `[Unreleased]`
- [ ] Matching logic implemented in `stellar-txwatch-core`
- [ ] `docs/alert-registry.md` rule descriptor table updated
- [ ] Contract unit test added
- [ ] Core integration test added
- [ ] TypeScript bindings regenerated (if the type surface changed)
```
