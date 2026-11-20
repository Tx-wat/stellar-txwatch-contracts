//! Tests for `set_alert_active`: a single-flag toggle that leaves rules, label,
//! webhook hash and tags untouched (#412).

#![cfg(test)]

use soroban_sdk::{
    symbol_short, testutils::Address as _, testutils::Events as _, vec, Address, Env, FromVal,
    IntoVal, String, Val, Vec,
};

use crate::{AlertRegistry, AlertRegistryClient, ContractError};

fn setup() -> (Env, Address, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, admin, client)
}

fn register(env: &Env, client: &AlertRegistryClient<'static>, owner: &Address) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "treasury watch"),
        &String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![env, String::from_str(env, "rule:transfer")],
        &Some(vec![env, String::from_str(env, "prod")]),
    )
}

#[test]
fn toggles_the_active_flag_both_ways() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let id = register(&env, &client, &owner);
    assert_eq!(client.get_alert_active(&id), Some(true));

    client.set_alert_active(&owner, &id, &false);
    assert_eq!(client.get_alert_active(&id), Some(false));
    assert!(!client.get_alert(&id).unwrap().active);

    client.set_alert_active(&owner, &id, &true);
    assert_eq!(client.get_alert_active(&id), Some(true));
    assert!(client.get_alert(&id).unwrap().active);
}

#[test]
fn leaves_rules_label_webhook_and_tags_unchanged() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let id = register(&env, &client, &owner);
    let before = client.get_alert(&id).unwrap();

    client.set_alert_active(&owner, &id, &false);
    let after = client.get_alert(&id).unwrap();

    assert_eq!(after.rules, before.rules);
    assert_eq!(after.label, before.label);
    assert_eq!(after.webhook_hash, before.webhook_hash);
    assert_eq!(after.tags, before.tags);
    assert_eq!(after.owner, before.owner);
    assert_eq!(after.target_contract, before.target_contract);
}

#[test]
fn non_owner_is_rejected_and_state_is_unchanged() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = register(&env, &client, &owner);

    let result = client.try_set_alert_active(&stranger, &id, &false);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));
    assert_eq!(client.get_alert_active(&id), Some(true));
}

#[test]
fn unknown_alert_returns_not_found() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let result = client.try_set_alert_active(&owner, &999, &false);
    assert_eq!(result, Err(Ok(ContractError::AlertNotFound.into())));
}

#[test]
fn rejected_while_paused() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let id = register(&env, &client, &owner);

    client.set_paused(&admin, &true);
    let result = client.try_set_alert_active(&owner, &id, &false);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused.into())));
    assert_eq!(client.get_alert_active(&id), Some(true));
}

#[test]
fn emits_alert_update_with_owner_and_new_flag() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let id = register(&env, &client, &owner);

    client.set_alert_active(&owner, &id, &false);

    // Read events before any other contract call: each invocation resets the buffer.
    let expected_topics: Vec<Val> = (symbol_short!("alert"), symbol_short!("update")).into_val(&env);
    let event = env
        .events()
        .all()
        .iter()
        .find(|(_, topics, _)| *topics == expected_topics)
        .expect("alert.update event");
    let data = <(u64, Address, bool)>::from_val(&env, &event.2);
    assert_eq!(data, (id, owner, false));
}
