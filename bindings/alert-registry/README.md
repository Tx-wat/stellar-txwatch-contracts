# @tx-wat/alert-registry-bindings

TypeScript bindings for the AlertRegistry Soroban smart contract.

## Installation

```bash
npm install @tx-wat/alert-registry-bindings
```

## Usage

```typescript
import { Contract, networks } from '@tx-wat/alert-registry-bindings';
import { SorobanRpc, Keypair } from '@stellar/stellar-sdk';

const rpcUrl = 'https://soroban-testnet.stellar.org';
const contractId = 'YOUR_CONTRACT_ID';
const keypair = Keypair.fromSecret('YOUR_SECRET_KEY');

const contract = new Contract({
  contractId,
  networkPassphrase: networks.testnet.networkPassphrase,
  rpcUrl,
});

// Register an alert
const result = await contract.register_alert({
  owner: keypair.publicKey(),
  target_contract: 'CONTRACT_ADDRESS_TO_WATCH',
  label: 'My Alert',
  webhook_hash: 'sha256_hash_of_webhook_url',
  rules: ['rule:transfer', 'rule:mint'],
  tags: ['treasury', 'high-value'], // optional; max 10 tags, 32 bytes each
}, {
  keypair,
});

console.log('Alert ID:', result);

// Get alerts for a contract
const alerts = await contract.get_alerts_for_contract({
  target_contract: 'CONTRACT_ADDRESS',
});

console.log('Alerts:', alerts);

// Get paginated alerts
const paginatedAlerts = await contract.get_alerts_by_owner_paginated({
  owner: keypair.publicKey(),
  offset: 0,
  limit: 10,
});

console.log('Paginated alerts:', paginatedAlerts);
```

## API

The bindings provide TypeScript types and methods for all AlertRegistry contract functions:

### Write Methods (require authentication)
- `register_alert` - Register a new alert configuration (optional `tags`)
- `update_tags` - Replace the tags of an alert (owner only; pass `undefined` to clear)
- `update_alert` - Update rules and active status
- `update_webhook` - Update webhook hash
- `remove_alert` - Remove an alert
- `initialize` - Initialize the contract with an admin (one-time)
- `transfer_admin` - Transfer admin role
- `set_per_owner_alert_limit` - Set per-owner alert limit (admin only)
- `remove_alert_by_admin` - Remove any alert (admin only)

### Read Methods
- `get_alert` - Get a single alert by ID
- `get_alerts_for_contract` - Get all alerts for a contract
- `get_alerts_by_owner` - Get all alerts owned by an address
- `get_contract_alerts_paginated` - Get paginated alerts for a contract
- `get_alerts_by_owner_paginated` - Get paginated alerts by owner
- `get_alert_count` - Get total number of alerts ever registered
- `get_active_alert_count` - Get number of active alerts for an owner
- `get_admin` - Get the current admin address
- `get_per_owner_alert_limit` - Get the per-owner alert limit

## Tags

`AlertConfig` carries an optional `tags` field: an array of up to 10 strings, each at
most 32 bytes, for client-side categorization. Pass them to `register_alert`, or replace
them later with `update_tags`. Only the owner may change an alert's tags.

```typescript
// Read the tags back from a stored alert
const { result: alert } = await contract.get_alert({ config_id: id });
console.log(alert?.tags); // string[] | undefined

// Replace the tags on an existing alert
await (await contract.update_tags({
  caller: keypair.publicKey(),
  config_id: id,
  tags: ['treasury'],
})).signAndSend();
```

Going over the limits is refused with `TooManyTags` or `TagTooLong`.

## Building from Source

```bash
# Build the contract first
cd ../..
cargo build --release --target wasm32-unknown-unknown

# Generate bindings
cd bindings/alert-registry
npm install
npm run build
```

## License

MIT

## TypeScript examples

These examples use the methods that the compiled `AlertRegistry` provides. The generated client
is built from the WASM, so re-check the exact method names and argument shapes after you
regenerate the bindings.

### 1. Set up a client

```typescript
import { Client, networks } from "@tx-wat/alert-registry-bindings";

const client = new Client({
  contractId: process.env.ALERT_REGISTRY_ID!,
  networkPassphrase: networks.testnet.networkPassphrase,
  rpcUrl: "https://soroban-testnet.stellar.org",
});
```

### 2. Register an alert

```typescript
const tx = await client.register_alert({
  owner,                 // G... address of the alert owner
  target_contract,       // C... contract the alert watches
  label: "Large transfer",
  webhook_hash: "hash",  // see docs/alert-registry.md for the hash scheme
  rules: ["rule:transfer"],
});
const { result: id } = await tx.signAndSend();
```

### 3. Update an alert

```typescript
await (await client.update_alert({
  caller: owner,
  config_id: id,
  rules: ["rule:transfer", "rule:mint"],
  active: true,
})).signAndSend();
```

### 4. Read an alert

```typescript
const { result: alert } = await client.get_alert({ config_id: id });
console.log(alert?.label, alert?.active);
```

### 5. Remove an alert

```typescript
await (await client.remove_alert({ caller: owner, config_id: id })).signAndSend();
```

Rule strings must be `rule:transfer` or `rule:mint`. Any other value is refused with
`InvalidRuleDescriptor`.
