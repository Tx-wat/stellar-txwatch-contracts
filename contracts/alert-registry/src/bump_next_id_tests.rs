//! Issue #496: `bump_alert` must keep the global ID counter alive, so a
//! registration made after a long gap still gets a fresh ID and cannot collide
//! with an existing alert.

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger as _}, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient, DEFAULT_TTL};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn advance_ledgers(env: &Env, by: u32) {
    env.ledger().with_mut(|l| l.sequence_number += by);
}

fn register(client: &AlertRegistryClient, env: &Env, owner: &Address, target: &Address, label: &str) -> u64 {
    client.register_alert(
        owner,
        target,
        &String::from_str(env, label),
        &String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

/// Register, bump just before the first TTL window closes, then advance past
/// where the original window would have ended. The next registration must get
/// the following ID (not 0), and the original alert must still be intact.
#[test]
fn registration_after_bump_past_original_ttl_gets_unique_id() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let first_id = register(&client, &env, &owner, &target, "first");
    assert_eq!(first_id, 0u64, "first registered alert must have id 0");

    // Bump while the original TTL window is still open.
    advance_ledgers(&env, DEFAULT_TTL - 10);
    client.bump_alert(&first_id, &DEFAULT_TTL);

    // Move past where the un-bumped window would have ended.
    advance_ledgers(&env, DEFAULT_TTL - 10);

    let second_id = register(&client, &env, &owner, &target, "second");
    assert_ne!(second_id, first_id, "new registration must not collide with existing alert 0");
    assert_eq!(second_id, 1u64, "counter must continue from 1 after the bump");
    assert_eq!(client.get_alert_count(), 2u64);

    let first = client.get_alert(&first_id).expect("bumped alert must still be stored");
    assert_eq!(first.label, String::from_str(&env, "first"), "original alert must not be overwritten");
}
