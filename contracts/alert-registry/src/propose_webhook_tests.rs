#![cfg(test)]
//! Tests for `propose_webhook` hash validation (#444).

use crate::*;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn hash64(env: &Env) -> String {
    String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

// ── propose_webhook hash validation (#444) ────────────────────────────

#[test]
fn test_propose_webhook_rejects_hash_shorter_than_64() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "A"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );
    let short = str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(short.len(), 63);

    assert_eq!(
        client
            .try_propose_webhook(&owner, &id, &short)
            .unwrap_err()
            .unwrap(),
        ContractError::InvalidWebhookHash
    );
    assert!(client.get_alert(&id).unwrap().pending_webhook_hash.is_none());
}

#[test]
fn test_propose_webhook_rejects_hash_longer_than_64() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "A"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );
    let long = str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(long.len(), 65);

    assert_eq!(
        client
            .try_propose_webhook(&owner, &id, &long)
            .unwrap_err()
            .unwrap(),
        ContractError::InvalidWebhookHash
    );
    assert!(client.get_alert(&id).unwrap().pending_webhook_hash.is_none());
}

#[test]
fn test_propose_webhook_accepts_valid_64_hash() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "A"),
        &hash64(&env),
        &vec![&env, str(&env, "rule:transfer")],
        &None,
    );
    let next = str(&env, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    client.try_propose_webhook(&owner, &id, &next).unwrap().unwrap();
    assert_eq!(client.get_alert(&id).unwrap().pending_webhook_hash, Some(next));
}
