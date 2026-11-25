//! Tests for `get_supported_rules` (#427).
//!
//! The registry must expose the list of valid rule descriptors so clients do not
//! have to read source code. The call is read-only and needs no auth.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup_without_auth() -> (Env, AlertRegistryClient<'static>) {
    // Deliberately no `mock_all_auths`: the listing must not require auth.
    let env = Env::default();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn hash64(env: &Env) -> String {
    String::from_str(env, "a3f1b2c4d5e6f7089abcdef01234567890abcdef01234567890abcdef012345")
}

#[test]
fn get_supported_rules_includes_transfer_and_mint() {
    let (env, client) = setup_without_auth();
    let rules = client.get_supported_rules();

    assert!(rules.contains(String::from_str(&env, "rule:transfer")));
    assert!(rules.contains(String::from_str(&env, "rule:mint")));
}

#[test]
fn get_supported_rules_has_no_duplicates() {
    let (env, client) = setup_without_auth();
    let rules = client.get_supported_rules();

    for i in 0..rules.len() {
        for j in (i + 1)..rules.len() {
            assert_ne!(rules.get(i).unwrap(), rules.get(j).unwrap());
        }
    }
    assert!(rules.len() >= 2);
}

#[test]
fn every_listed_rule_is_accepted_by_register_alert() {
    let (env, client) = setup_without_auth();
    env.mock_all_auths();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    for rule in client.get_supported_rules().iter() {
        let id = client.register_alert(
            &owner,
            &target,
            &String::from_str(&env, "rule check"),
            &hash64(&env),
            &vec![&env, rule.clone()],
            &None,
        );
        assert!(client.get_alert(&id).is_some());
    }
}
