//! Unit tests covering the validations added by issues #206, #207, #208, and #209.
//!
//! - #206: empty `rules` vec is rejected with `EmptyRules`.
//! - #207: duplicate rule descriptors are rejected with `DuplicateRule`.
//! - #208: `webhook_hash` must be exactly 64 characters (`InvalidWebhookHash`).
//! - #209: `limit` in paginated queries is clamped to `MAX_BATCH_SIZE`.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient, ContractError, MAX_BATCH_SIZE};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

/// Return a valid 64-character SHA-256 hex placeholder for tests.
fn hash64(env: &Env) -> String {
    String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

// ── #206: EmptyRules ──────────────────────────────────────────────────────────

/// register_alert with an empty rules vec returns EmptyRules.
#[test]
fn register_alert_empty_rules_returns_empty_rules_error() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env],
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::EmptyRules)));
}

/// update_alert with an empty rules vec returns EmptyRules.
#[test]
fn update_alert_empty_rules_returns_empty_rules_error() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    // Register successfully first
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    // Now try to update with empty rules
    let result = client.try_update_alert(
        &owner,
        &id,
        &vec![&env], // empty rules
        &true,
    );

    assert_eq!(result, Err(Ok(ContractError::EmptyRules)));
}

// ── #207: DuplicateRule ───────────────────────────────────────────────────────

/// register_alert with duplicate rule descriptors returns DuplicateRule.
#[test]
fn register_alert_duplicate_rules_returns_duplicate_rule_error() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![
            &env,
            str(&env, "rule:transfer"),
            str(&env, "rule:transfer"), // duplicate
        ],
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::DuplicateRule)));
}

/// update_alert with duplicate rule descriptors returns DuplicateRule.
#[test]
fn update_alert_duplicate_rules_returns_duplicate_rule_error() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    let result = client.try_update_alert(
        &owner,
        &id,
        &vec![
            &env,
            str(&env, "rule:mint"),
            str(&env, "rule:mint"), // duplicate
        ],
        &true,
    );

    assert_eq!(result, Err(Ok(ContractError::DuplicateRule)));
}

/// Non-duplicate rules are accepted normally.
#[test]
fn register_alert_non_duplicate_rules_succeeds() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![
            &env,
            str(&env, "rule:transfer"),
            str(&env, "rule:mint"), // different — OK
        ],
        &None,
    );

    assert!(result.is_ok(), "distinct rules should be accepted");
}

// ── #208: InvalidWebhookHash ─────────────────────────────────────────────────

/// register_alert with a hash shorter than 64 characters returns InvalidWebhookHash.
#[test]
fn register_alert_short_hash_returns_invalid_webhook_hash() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "tooshort"), // not 64 chars
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

/// register_alert with an empty hash returns InvalidWebhookHash.
#[test]
fn register_alert_empty_hash_returns_invalid_webhook_hash() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, ""), // empty string
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

/// register_alert with a hash longer than 64 characters returns InvalidWebhookHash.
#[test]
fn register_alert_long_hash_returns_invalid_webhook_hash() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    // 65 chars
    let long_hash = str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &long_hash,
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

/// register_alert with exactly 64 characters accepts the hash.
#[test]
fn register_alert_valid_64_char_hash_succeeds() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    assert!(result.is_ok(), "64-char hash must be accepted");
}

/// update_webhook with a short hash returns InvalidWebhookHash.
#[test]
fn update_webhook_short_hash_returns_invalid_webhook_hash() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    let result = client.try_update_webhook(
        &owner,
        &id,
        &str(&env, "short"), // not 64 chars
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

/// update_webhook with exactly 64 characters succeeds.
#[test]
fn update_webhook_valid_hash_succeeds() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    // A different valid 64-char hash
    let new_hash = str(&env, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let result = client.try_update_webhook(&owner, &id, &new_hash);
    assert!(result.is_ok(), "valid 64-char hash must be accepted");
}

// ── #398: batch_register_alerts all-or-nothing atomicity ─────────────────────

/// A batch whose second element has an invalid (63-char) webhook hash must
/// fail with `InvalidWebhookHash` and register none of the alerts — including
/// the valid first element.
#[test]
fn batch_register_alerts_invalid_hash_rolls_back_whole_batch() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    // limit = 1000 must be clamped to MAX_BATCH_SIZE
    let page = client
        .get_contract_alerts_paginated(&querier, &target, &0u32, &1000u32)
        ;

    let result = client.try_update_tags(&owner, &id, &tags);
    assert_eq!(result, Err(Ok(ContractError::TooManyTags)));
}

/// update_tags with a single 33-byte tag returns TagTooLong.
#[test]
fn update_tags_tag_too_long_returns_tag_too_long_error() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    let page = client
        .get_alerts_by_owner_paginated(&querier, &owner, &0u32, &u32::MAX)
        ;

    // The valid first element must not have been persisted.
    assert_eq!(
        client.count_for_contract(&target_a),
        0,
        "first element must not be registered when the batch fails"
    );
}

/// update_tags with exactly 10 tags of 32 bytes each succeeds.
#[test]
fn update_tags_max_tags_and_length_succeeds() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );

    let page = client
        .get_contract_alerts_paginated(&querier, &target, &0u32, &5u32)
        ;

    let result = client.try_update_tags(&owner, &id, &tags);
    assert!(result.is_ok(), "10 tags of 32 bytes must be accepted");
}

// ── #456: register_alert and update_label share the same label boundary ───────

/// A label of exactly `len` bytes, built from raw bytes so the test has no string-literal limits.
fn label_of_len(env: &Env, len: usize) -> String {
    let bytes = [b'a'; 200];
    String::from_bytes(env, &bytes[..len])
}

/// The 128-byte label boundary is enforced identically by register_alert and update_label.
#[test]
fn register_and_update_label_share_128_byte_boundary() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let rules = vec![&env, str(&env, "rule")];

    // Exactly at the boundary: both entry points accept the label.
    let at_limit = label_of_len(&env, 128);
    let id = client.register_alert(&owner, &target, &at_limit, &hash64(&env), &rules, &None);
    assert_eq!(client.try_update_label(&owner, &id, &at_limit), Ok(Ok(())));

    // One byte over the boundary: both entry points reject with LabelTooLong.
    let over_limit = label_of_len(&env, 129);
    let register_result = client.try_register_alert(
        &owner,
        &target,
        &over_limit,
        &hash64(&env),
        &rules,
        &None,
    );
    let update_result = client.try_update_label(&owner, &id, &over_limit);

    assert_eq!(register_result, Err(Ok(ContractError::LabelTooLong)));
    assert_eq!(update_result, Err(Ok(ContractError::LabelTooLong)));
}
