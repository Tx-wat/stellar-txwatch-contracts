# AlertRegistry Event Listener

`index.ts` polls Soroban RPC for events emitted by the `AlertRegistry` contract and prints each one as a JSON line. It keeps a paging cursor, so a restart resumes where it stopped.

## Prerequisites

- **Node.js** 18 or later (LTS).
- **`@stellar/stellar-sdk`** 12.x. The repo's bindings pin `12.3.0` (`bindings/watcher-registry/package.json`). Install it with `npm install @stellar/stellar-sdk@^12`.
- **`tsx`** or another TypeScript runner, for example `npx tsx index.ts`.
- A deployed **AlertRegistry** contract ID. See `DEPLOYMENTS.md` at the repo root for the current testnet address.

## Environment variables

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `ALERT_REGISTRY_ID` | Yes | none | AlertRegistry contract address (`C...`, 56 characters). The script throws on startup if it is missing. |
| `SOROBAN_RPC_URL` | No | `https://soroban-testnet.stellar.org` | Soroban RPC endpoint to poll. |

## Events subscribed to

The listener subscribes to one event. It filters by contract ID and by the topic pair `("alert", "register")`.

| Topics | Emitted by | Data tuple `(id, owner, target_contract)` | Notes |
|---|---|---|---|
| `["alert", "register"]` | `register_alert`, `batch_register_alerts` | `[id: u64, owner: Address, target_contract: Address]` | One event per new alert. |

After `scValToNative`, `ev.value` becomes a positional array. `id` is a `bigint` in JavaScript, and the two addresses are `C...` (contract) or `G...` (account) strings.

Each printed line has this shape:

```json
{"ledger":123456,"topics":["alert","register"],"data":[1,"G...OWNER","C...TARGET"]}
```

Events for other actions (`update`, `remove`, `wh_prop`, `webhook`, `bump`) are not subscribed to. To handle them, add their topic pairs to the `topics` filter in `poll()`, and give each one its own data shape. The event reference in `docs/events.md` lists them.

## Handler contract

- `handle(ev)` is called once per event. Replace its body with real logic.
- Handlers must be **idempotent**. A crash between handling an event and saving the cursor replays a few events on restart.
- The cursor is held in memory only (`let cursor`). Persist it to disk or a database if you need restarts to resume without replaying from ledger 0.

## Running against testnet

```bash
cd examples/event-listener
npm install @stellar/stellar-sdk@^12 tsx
ALERT_REGISTRY_ID=<C...> \
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org \
npx tsx index.ts
```

Testnet addresses reset periodically. If the listener finds no events, check that `ALERT_REGISTRY_ID` matches the current deployment in `DEPLOYMENTS.md`.

Network passphrase for reference: `Test SDF Network ; September 2015`.
