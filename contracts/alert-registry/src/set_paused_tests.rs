#![cfg(test)]
//! Tests for `set_paused` (#445): pausing blocks mutations, but the admin
//! must always be able to unpause.

use crate::*;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

fn setup() -> (Env, AlertRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn hash64(env: &Env) -> String {
    String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

#[test]
fn test_admin_can_unpause_from_paused_state() {
    let (_env, client, admin) = setup();
    client.set_paused(&admin, &true);
    assert!(client.is_paused());

    client.set_paused(&admin, &false);
    assert!(!client.is_paused());
}

#[test]
fn test_pause_blocks_mutations_until_admin_unpauses() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let rules = vec![&env, str(&env, "rule:transfer")];

    client.set_paused(&admin, &true);
    assert_eq!(
        client
            .try_register_alert(&owner, &target, &str(&env, "A"), &hash64(&env), &rules, &None)
            .unwrap_err()
            .unwrap(),
        ContractError::ContractPaused
    );

    client.set_paused(&admin, &false);
    client
        .try_register_alert(&owner, &target, &str(&env, "A"), &hash64(&env), &rules, &None)
        .unwrap()
        .unwrap();
}
