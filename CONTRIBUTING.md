# Contributing

## Prerequisites

Install the Rust toolchain and Soroban target:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

Install the Stellar CLI:

```bash
cargo install --locked stellar-cli --features opt
```

## Formatting

Run `cargo fmt` before pushing. CI enforces formatting with `cargo fmt --all -- --check`, so unformatted code will fail the build. Formatting settings live in `rustfmt.toml`.

### Pre-commit hooks (recommended)

The repo ships a [`.pre-commit-config.yaml`](.pre-commit-config.yaml) that runs the same `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` checks CI uses, so problems surface before you commit:

```bash
pip install pre-commit
pre-commit install
```

Run the hooks against the whole tree at any time with `pre-commit run --all-files`.

## Build

```bash
cargo build --release --target wasm32-unknown-unknown
```

## Test

```bash
cargo test
```

Tests run natively (no WASM target needed). Each contract has a `#[cfg(test)]` module covering happy paths, unauthorized rejections, and edge cases.

## Pull Requests

When opening a pull request, use the template in [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md). It includes contract-specific checklist items (tests, CHANGELOG, docs, TypeScript bindings, clippy, `require_auth()` verification, and `ContractError` rustdoc) that must be completed for contract changes.

## Deploy to Testnet

1. Set up a funded testnet identity:

```bash
stellar keys generate deployer --network testnet
stellar keys fund deployer --network testnet
```

2. Run the deploy script:

```bash
bash scripts/deploy.sh
```

3. Update `DEPLOYMENTS.md` with the printed contract addresses.

## Changelog Updates

Every PR that changes contract behavior, adds a function, or fixes a bug must
include a `CHANGELOG.md` update. Read-only changes (docs, tests, CI) are
optional but encouraged.

1. Add entries under the `[Unreleased]` section.
2. Use one of the categories: **Added**, **Fixed**, **Changed**, **Deprecated**,
   **Removed**, **Security**.
3. Reference the issue number at the end of the entry, e.g. `(#123)`.

Example entry:

```markdown
## [Unreleased]

### Added
- `rule:burn` descriptor accepted by `validate_rules` (#123)

### Fixed
- Reject unauthorized `set_admin` calls (#124)
```

## Adding a New Function to an Existing Contract

1. Add the function inside the `#[contractimpl]` block in `contracts/<name>/src/lib.rs`.
2. If it mutates state, call `<caller>.require_auth()` as the first line.
3. Add at least one test in the `#[cfg(test)]` module covering the happy path and any auth rejection.
4. Run `cargo fmt` and `cargo test` to confirm everything passes.
5. Update the relevant doc in `docs/`.

## Adding a New Rule Descriptor

Rule descriptors (e.g. `rule:invoke`, `rule:burn`) are the identifiers that
contracts emit and the off-chain watcher matches against. Adding a new one
spans several layers, so follow the full process below. See
[docs/contributing-contracts.md](docs/contributing-contracts.md#adding-a-new-rule-descriptor)
for the detailed guide.

1. **Propose** — open an issue describing the descriptor name, its semantics,
   and the use case it enables. Get agreement before writing code.
2. **Contract change** — in `contracts/alert-registry/src/lib.rs`, add the descriptor
   to the allow-list in `validate_rule` (not `validate_rules`, which only loops over
   the entries and checks length and duplicates). There is no `get_supported_rules`.
   The full walkthrough, including the positive, near-miss, and duplicate tests, is in
   [docs/contributing-contracts.md](docs/contributing-contracts.md#adding-a-new-rule-descriptor).
3. **Watcher change** — implement the matching logic for the descriptor in
   [`stellar-txwatch-core`](https://github.com/Tx-wat/stellar-txwatch-core).
4. **Docs** — update the rule descriptor table in `docs/alert-registry.md`.
5. **Test** — add a unit test in the contract and an integration test in the
   core watcher.
6. **TypeScript** — regenerate the bindings if the type surface changes.

### PR Template for New Rule Descriptors

When opening a PR that adds a rule descriptor, use this checklist in the PR
body:

```markdown
## New Rule Descriptor: `rule:<name>`

- [ ] Proposal issue linked (name, semantics, use case)
- [ ] Descriptor added to `validate_rules` allow-list
- [ ] Matching logic implemented in `stellar-txwatch-core`
- [ ] `docs/alert-registry.md` rule descriptor table updated
- [ ] Contract unit test added
- [ ] Core integration test added
- [ ] TypeScript bindings regenerated (if type surface changed)
```

## Sister Repos

- **Core engine:** https://github.com/Tx-wat/stellar-txwatch-core
- **Web dashboard:** https://github.com/Tx-wat/stellar-txwatch-web
