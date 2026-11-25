//! Tests for `get_alert_count_for_contract`: the live alert count for one
//! target contract, which must agree with `get_alert_ids_for_contract`.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn register_for(
    env: &Env,
    client: &AlertRegistryClient<'static>,
    owner: &Address,
    target: &Address,
) -> u64 {
    client.register_alert(
        owner,
        target,
        &String::from_str(env, "alert"),
        &String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn zero_for_a_contract_with_no_alerts() {
    let (env, client) = setup();
    let target = Address::generate(&env);
    assert_eq!(client.get_alert_count_for_contract(&target), 0);
}

#[test]
fn counts_only_alerts_for_the_given_target() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let watched = Address::generate(&env);
    let other = Address::generate(&env);

    register_for(&env, &client, &owner, &watched);
    register_for(&env, &client, &owner, &watched);
    register_for(&env, &client, &owner, &other);

    assert_eq!(client.get_alert_count_for_contract(&watched), 2);
    assert_eq!(client.get_alert_count_for_contract(&other), 1);
}

#[test]
fn removed_alerts_are_not_counted() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let first = register_for(&env, &client, &owner, &target);
    register_for(&env, &client, &owner, &target);
    assert_eq!(client.get_alert_count_for_contract(&target), 2);

    client.remove_alert(&owner, &first);
    assert_eq!(client.get_alert_count_for_contract(&target), 1);
}

#[test]
fn matches_the_length_of_the_id_list() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    for _ in 0..5 {
        register_for(&env, &client, &owner, &target);
    }
    let ids = client.get_alert_ids_for_contract(&target);
    assert_eq!(client.get_alert_count_for_contract(&target), ids.len());
}
