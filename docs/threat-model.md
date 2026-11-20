# Threat Model: Watcher Authorization System

## Overview

The `WatcherRegistry` contract stores a set of authorized watcher node addresses on-chain. Only addresses registered by an admin may act as watchers in the Tx-wat system.

The `AlertRegistry` contract stores per-owner alert configurations (webhook hashes and rule definitions) that off-chain watchers poll to decide what to monitor. This document covers the threat analysis for both registries.

---

## Assets

| Asset | Description |
|---|---|
| Watcher registry | The on-chain list of authorized watcher addresses |
| Admin authority | The ability to add/remove watchers and transfer admin |
| Alert configurations | Per-owner alert rules and webhook hashes stored in `AlertRegistry` |
| Owner index | The mapping of owners to their registered alert IDs |

---

## Trust Assumptions

- **Admin keypair is secure.** The admin Stellar account is assumed to be controlled by a trusted operator. Compromise of the admin key is out of scope for the contract itself.
- **Stellar protocol integrity.** The contract relies on `require_auth()` from the Soroban SDK. It trusts that the Stellar network correctly enforces signature verification.
- **Soroban re-entrancy model.** Soroban executes contract calls atomically and does not support classic callback-based re-entrancy into the same stateful contract within a single transaction. The registry contracts do not invoke other contracts during state mutation, so cross-contract callers cannot cause re-entrant state changes.
- **Watcher nodes are honest once authorized.** The contract only controls *who* may be a watcher, not *what* an authorized watcher does off-chain.
- **Owners are responsible for their own alert rules.** The `AlertRegistry` validates structure but cannot judge whether a rule is semantically meaningful or safe to evaluate.

---

## What the Contract Protects Against

- **Unauthorized watcher registration.** Only the current admin can call `register_watcher` or `remove_watcher`. Any unsigned or incorrectly signed call is rejected at the protocol level.
- **Admin hijacking via direct call.** `transfer_admin` requires the current admin's auth signature, preventing an attacker from reassigning admin without controlling the current admin key.
- **Replay attacks.** Stellar's sequence number mechanism prevents replaying previously valid transactions.
- **Unauthorized alert mutation.** `AlertRegistry` mutations require the owning account's `require_auth()`, so only the owner can create, update, or remove their own alerts.

---

## What the Contract Does NOT Protect Against

- **Compromised admin key.** If the admin keypair is stolen, an attacker can register arbitrary watchers or transfer admin to themselves. No multi-sig or time-lock is enforced at the contract level.
- **Malicious behavior by authorized watchers.** Once a watcher is registered, the contract has no visibility into what that node does off-chain (e.g., sending false alerts, ignoring events).
- **Front-running.** Because Stellar transactions are public before finalization, an observer could attempt to front-run an admin action, though the practical impact is low given the permissioned nature of the registry.
- **Social engineering of the admin.** The contract cannot prevent an admin from being tricked into registering a malicious watcher address.
- **Denial of service.** A malicious admin (or compromised key) could remove all watchers, halting the monitoring system. No minimum-watcher-count enforcement exists.
- **Semantic validity of alert rules.** `AlertRegistry` cannot determine whether a rule is well-formed for the off-chain evaluator; a structurally valid but pathological rule may still burden watchers.
- **Off-chain cache staleness.** The contract cannot force watchers to re-read state; watchers must poll or subscribe to events to stay current.

---

## Attack Scenarios

### 1. Attacker tries to register themselves as a watcher
**Vector:** Call `register_watcher` without admin auth.  
**Outcome:** Rejected by `require_auth()`. No state change.

### 2. Admin key is compromised
**Vector:** Attacker obtains the admin private key and calls `register_watcher` or `transfer_admin`.  
**Outcome:** Attacker gains full control of the registry. **Mitigation outside contract scope** — use hardware wallets, multi-sig accounts, or key rotation procedures.

### 3. Authorized watcher goes rogue
**Vector:** A legitimately registered watcher node starts sending false or malicious alerts.  
**Outcome:** The contract cannot detect this. **Mitigation:** Admin removes the watcher via `remove_watcher`; off-chain monitoring of watcher behavior is required.

### 4. Admin removes all watchers (accidental or malicious)
**Vector:** Admin calls `remove_watcher` for every registered address.  
**Outcome:** No watchers remain; the monitoring system stops functioning. **Mitigation outside contract scope** — operational procedures and alerts on registry changes.

---

## AlertRegistry Attack Scenarios

### A1. Owner registers alerts with invalid rules to DoS the watcher
**Vector:** An owner submits `register_alert` with rule definitions that are structurally accepted on-chain but pathological for the off-chain evaluator (e.g., extremely large rule sets, deeply nested conditions, or rules that trigger excessive computation).  
**Outcome:** Watchers that naively evaluate every rule may stall, drop other owners' alerts, or exhaust their compute budget — degrading monitoring for all users.  
**Mitigation:** `AlertRegistry` enforces per-alert size and rule-count bounds at registration time. Off-chain watchers must additionally apply evaluation timeouts and per-owner quotas, and isolate rule evaluation so one owner's rules cannot starve others. Tracked as a follow-up issue for watcher-side rule sandboxing.

### A2. Owner registers maximum number of alerts to fill owner index and exhaust storage budget
**Vector:** An owner repeatedly calls `register_alert` up to the per-owner cap, inflating their entry in the owner index and consuming contract storage (and the associated rent budget).  
**Outcome:** Storage costs rise and the owner index grows, increasing iteration cost for watchers that scan all alerts. If the cap is too high, a single owner can crowd out others' visibility.  
**Mitigation:** `AlertRegistry` enforces a hard per-owner alert cap and rejects registrations beyond it. Watchers should paginate the owner index rather than loading it wholesale. Tracked as a follow-up issue to review the cap value against expected storage rent.

### A3. Attacker front-runs an alert registration to claim a target contract before the legitimate owner
**Vector:** An attacker observes a pending `register_alert` for a target contract and submits their own registration for the same target with a higher fee, hoping to be ordered first.  
**Outcome:** Because alerts are keyed by owner, the attacker's registration does not overwrite the legitimate owner's entry — both coexist. The realistic impact is limited to watcher noise and index inflation, not theft of the alert.  
**Mitigation:** Alerts are namespaced per owner and mutations require the owner's `require_auth()`, so an attacker cannot claim another owner's alert slot. Watchers should treat duplicate target registrations as independent and deduplicate off-chain.

### A4. Admin abuses `remove_alert_by_admin` to delete legitimate alerts without owner consent
**Vector:** A malicious or compromised admin calls `remove_alert_by_admin` on an owner's alert.  
**Outcome:** The owner's alert is silently removed, disabling monitoring for that owner without their knowledge.  
**Mitigation:** Admin removal is intended only for abuse response. Every admin removal should emit an event so owners and watchers can detect it, and operational procedures should require logging and review. Tracked as a follow-up issue to add an audit trail / event for admin removals.

### A5. Off-chain watcher caches stale alert data after an owner updates rules
**Vector:** An owner calls `update_alert` to change rules or webhook hash, but a watcher continues evaluating the cached pre-update configuration.  
**Outcome:** The watcher fires alerts based on outdated rules or posts to a stale webhook, causing missed or misdirected notifications.  
**Mitigation:** `AlertRegistry` emits an event on every alert mutation. Watchers must subscribe to these events (or re-read state on a bounded interval) and invalidate their cache on update. Tracked as a follow-up issue to document the watcher cache-invalidation contract.

### A6. ID counter overflow at `u64::MAX`
**Vector:** The alert ID counter is incremented on every registration. If it ever reaches `u64::MAX`, the next increment overflows.  
**Outcome:** Theoretical only — reaching `u64::MAX` registrations is infeasible in practice. If it occurred, registration would panic or wrap depending on the arithmetic used, potentially colliding IDs.  
**Mitigation:** Registration uses checked arithmetic so overflow panics rather than wrapping, preventing ID collisions. The bound is documented here for completeness; no operational action is required.

---

## WatcherRegistry Threat Model

This section covers threats specific to `WatcherRegistry`. Each threat lists the attack, the contract behavior, and the residual risk.

### W1. Unauthorized watcher registration

**Vector:** An attacker calls `register_watcher`, `batch_register_watchers`, or `replace_watcher` to add their own address to the watcher set.
**Contract behavior:** Each of these functions requires admin authorization through `require_auth()`. A call without a valid admin signature is rejected before any state changes.
**Residual risk:** None at the contract level, as long as the admin key is intact. An attacker who holds the admin key is covered by W2.

### W2. Admin key compromise

**Vector:** The admin key is stolen. The attacker can then register arbitrary watchers, call `clear_all_watchers`, `transfer_admin`, or `add_admin`, or pause the registry.
**Contract behavior:** The contract does not limit what the admin can do. Multi-admin mode (`add_admin`) does not change this, because any admin has full authority. `remove_admin` refuses to remove the last admin (`LastAdmin`), so the registry is never left without one.
**Mitigation outside contract scope:** Hardware wallets or multi-sig admin accounts, plus a documented key-rotation procedure. Monitor `admin` events (`add`, `remove`, `transfer`) so an unexpected change is noticed quickly.
**Residual risk:** High. A single compromised admin key gives full control of the watcher set.

### W3. Watcher list exhaustion (`MAX_WATCHERS`)

**Vector:** An attacker with admin access (W2) or a runaway admin script fills the watcher list up to `MAX_WATCHERS` (1,000). Once full, further `register_watcher` calls return `WatcherLimitExceeded`. `batch_register_watchers` checks the combined total the same way.
**Contract behavior:** The cap is enforced on every registration path, so the list cannot grow beyond 1,000 entries. Reads and `is_authorized` are not affected.
**Operational effect:** Legitimate watchers cannot be added until existing entries are removed. Removing a stale watcher with `remove_watcher` frees a slot.
**Mitigation:** Watchers should renew (`renew_watcher`) before expiry, and operators should remove decommissioned watchers promptly. Alert when `get_watcher_count` approaches the cap.
**Residual risk:** Medium. The cap bounds storage cost, but a full list blocks onboarding.

### W4. Paused-registry denial of service

**Vector:** The admin calls `set_paused(true)`, either by mistake or after a compromise (W2).
**Contract behavior:** While paused, every state-changing function returns `ContractPaused`. These include `register_watcher`, `remove_watcher`, `replace_watcher`, `batch_register_watchers`, `renew_watcher`, `clear_all_watchers`, `add_admin`, `remove_admin`, and `transfer_admin`. Read functions and `is_authorized` / `is_watcher_authorized` keep working, so watchers that are already registered continue to operate.
**Impact:** Watcher membership cannot change during a pause. A paused registry does not stop existing watchers from being authorized, but it does block emergency removal of a rogue watcher.
**Mitigation:** Pause only with a documented reason, and unpause as soon as the incident is resolved. Keep an admin key available to unpause even when the primary admin is unavailable (see W2).
**Residual risk:** Medium. A pause can freeze the response to an active incident, so pausing should be a deliberate, logged decision.

## Security Properties Summary

| Property | Enforced by contract |
|---|---|
| Only admin can modify the registry | ✅ |
| Admin transfer requires current admin auth | ✅ |
| Replay protection | ✅ (Stellar protocol) |
| Only owner can mutate their own alerts | ✅ |
| Per-owner alert cap enforced | ✅ |
| Alert ID overflow prevented (checked arithmetic) | ✅ |
| Admin key compromise protection | ❌ |
| Off-chain watcher behavior enforcement | ❌ |
| Multi-sig / time-lock on admin actions | ❌ |
| Semantic validity of alert rules | ❌ |
| Off-chain cache freshness | ❌ |
