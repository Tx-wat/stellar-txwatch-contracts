//! Tests for the shared alert removal path (#438).
//!
//! Owner-initiated and admin-initiated removals both go through
//! `remove_alert_record`; these tests pin the event payload each one emits
//! and check that the owner and contract indexes are cleaned up.

#![cfg(test)]

use soroban_sdk::{
    symbol_short, testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, String,
};

use crate::{AlertRegistry, AlertRegistryClient};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn setup() -> (Env, AlertRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn register(env: &Env, client: &AlertRegistryClient, owner: &Address, target: &Address) -> u64 {
    client.register_alert(
        owner,
        target,
        &String::from_str(env, "alert"),
        &String::from_str(env, HASH),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn owner_removal_emits_id_and_caller() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = register(&env, &client, &owner, &target);

    client.remove_alert(&owner, &id);

    let (_, topics, data) = env.events().all().last().unwrap();
    assert_eq!(topics, (symbol_short!("alert"), symbol_short!("remove")).into_val(&env));
    let payload: (u64, Address) = data.into_val(&env);
    assert_eq!(payload, (id, owner.clone()));
}

#[test]
fn admin_removal_emits_id_admin_and_owner() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = register(&env, &client, &owner, &target);

    client.remove_alert_by_admin(&admin, &id);

    let (_, topics, data) = env.events().all().last().unwrap();
    assert_eq!(topics, (symbol_short!("alert"), symbol_short!("remove")).into_val(&env));
    let payload: (u64, Address, Address) = data.into_val(&env);
    assert_eq!(payload, (id, admin.clone(), owner.clone()));
}

#[test]
fn owner_removal_clears_owner_and_contract_indexes() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = register(&env, &client, &owner, &target);
    assert_eq!(client.get_alert_ids_by_owner(&owner).len(), 1);
    assert_eq!(client.get_alert_ids_for_contract(&target).len(), 1);

    client.remove_alert(&owner, &id);

    assert_eq!(client.get_alert_ids_by_owner(&owner).len(), 0);
    assert_eq!(client.get_alert_ids_for_contract(&target).len(), 0);
}

#[test]
fn admin_removal_clears_owner_and_contract_indexes() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = register(&env, &client, &owner, &target);

    client.remove_alert_by_admin(&admin, &id);

    assert_eq!(client.get_alert_ids_by_owner(&owner).len(), 0);
    assert_eq!(client.get_alert_ids_for_contract(&target).len(), 0);
}
