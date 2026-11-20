# Deployments

This file tracks deployed contract addresses for each network.  
After running `scripts/deploy.sh`, replace the placeholder values with the printed addresses and commit the update.

> **Address format:** Stellar contract addresses are 56-character strings beginning with `C`  
> (e.g. `CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4`).  
> Testnet addresses are only valid until the next testnet reset.

---

## Stellar Testnet

| Contract | Address |
|---|---|
| Alert Registry | `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |
| Watcher Registry | `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |

### Testnet Network Details

| Parameter | Value |
|---|---|
| Network | Testnet |
| RPC URL | `https://soroban-testnet.stellar.org` |
| Network Passphrase | `Test SDF Network ; September 2015` |
| Horizon URL | `https://horizon-testnet.stellar.org` |
| Deploy date | _Pending next testnet deployment_ |
| Rust toolchain | `1.88.0` (from `rust-toolchain.toml`) |
| Soroban SDK | `23.0.0` (from `Cargo.toml`) |

> Testnet resets periodically. Re-deploy with `bash scripts/deploy.sh` and update the addresses above after each reset.

---

## Stellar Mainnet

| Contract | Address |
|---|---|
| Alert Registry | `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |
| Watcher Registry | `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |

### Mainnet Network Details

| Parameter | Value |
|---|---|
| Network | Mainnet (Public) |
| RPC URL | `https://mainnet.stellar.validationcloud.io/v1/<API_KEY>` |
| Network Passphrase | `Public Global Stellar Network ; September 2015` |
| Horizon URL | `https://horizon.stellar.org` |

> Mainnet has not been deployed yet. Replace placeholders and remove this note once a production deployment is made.

---

## Expected WASM Hashes

`scripts/verify.sh` compares the on-chain WASM hash of each deployed contract against the values below.  
After building with `cargo build --target wasm32-unknown-unknown --release`, record the hash of each artifact:

```bash
sha256sum target/wasm32-unknown-unknown/release/*.wasm
```

| Contract | WASM Hash (sha256) |
|---|---|
| Alert Registry | `XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |
| Watcher Registry | `XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |

> Replace the `XXXX...` placeholders with the hashes of the deployed artifacts and commit the update alongside the addresses.

---

## How to Update This File

1. Run the deploy script:
   ```bash
   bash scripts/deploy.sh
   ```
2. Copy the contract addresses printed at the end of the script output.
3. Replace the corresponding `CXXX...` placeholders in the table above.
4. Record the deployed WASM hashes in the **Expected WASM Hashes** table above.
5. Commit the update:
   ```bash
   git add DEPLOYMENTS.md
   git commit -m "deploy: update contract addresses for <network> (<date>)"
   ```

---

## Deployment History

| Date | Network | Contract | Address | Notes |
|---|---|---|---|---|
| — | — | — | — | Initial placeholder |
