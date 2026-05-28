# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `CHANGELOG.md` to track version history (#75)
- `SECURITY.md` with responsible disclosure policy (#76)
- `docs/ttl.md` documenting TTL values and their implications (#77)
- Inline rustdoc comments on all public and key private functions (#78)
- Expanded `.gitignore` to exclude build artifacts and test snapshots

## [0.1.0] - 2025-05-28

### Added
- `AlertRegistry` contract: register, update, remove, and query alert configs on-chain
- `WatcherRegistry` contract: manage authorized watcher node addresses with admin controls
- Persistent storage with TTL extension on every write
- Owner-keyed and contract-keyed index lookups for alert configs
- Stellar CLI, JavaScript SDK, and Rust SDK usage examples in `README.md`
- Deployment addresses tracked in `DEPLOYMENTS.md`
- Function reference docs in `docs/alert-registry.md` and `docs/watcher-registry.md`
- Contribution guidelines in `CONTRIBUTING.md`

[Unreleased]: https://github.com/Tx-wat/stellar-txwatch-contracts/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Tx-wat/stellar-txwatch-contracts/releases/tag/v0.1.0
