//! `validate_alert_input` dry-run: same checks as `register_alert`, no auth,
//! no storage writes (#417).

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

use crate::{AlertRegistry, AlertRegistryClient, ContractError};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn s(env: &Env, text: &str) -> String {
    String::from_str(env, text)
}

#[test]
fn valid_input_passes() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );
    assert_eq!(result, Ok(Ok(())));
}

#[test]
fn does_not_write_storage() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    client.validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );
    assert_eq!(client.get_alert_count(), 0);
    assert_eq!(client.get_active_alert_count(&owner), 0);
    assert_eq!(client.get_alerts_by_owner(&owner).len(), 0);
}

#[test]
fn does_not_require_auth() {
    // No mock_all_auths: any require_auth call would panic.
    let env = Env::default();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:mint")],
        &None,
    );
    assert_eq!(result, Ok(Ok(())));
}

#[test]
fn rejects_bad_webhook_hash_length() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, "short"),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

#[test]
fn rejects_label_over_128_bytes() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let bytes = [b'x'; 129];
    let long = core::str::from_utf8(&bytes).unwrap();
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, long),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::LabelTooLong)));
}

#[test]
fn rejects_empty_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let empty: Vec<String> = Vec::new(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &empty,
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::EmptyRules)));
}

#[test]
fn rejects_unknown_rule_descriptor() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:bogus")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidRuleDescriptor)));
}

#[test]
fn rejects_duplicate_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:mint"), s(&env, "rule:mint")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::DuplicateRule)));
}

#[test]
fn rejects_too_many_tags() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let mut tags: Vec<String> = Vec::new(&env);
    for _ in 0..=crate::MAX_TAGS {
        tags.push_back(s(&env, "tag"));
    }
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &Some(tags),
    );
    assert_eq!(result, Err(Ok(ContractError::TooManyTags)));
}

#[test]
fn reports_per_owner_limit_without_pruning_or_registering() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    client.initialize(&admin);
    client.set_per_owner_alert_limit(&admin, &1);

    client.register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );

    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, HASH),
        &vec![&env, s(&env, "rule:transfer")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::OwnerAlertLimitExceeded)));
    assert_eq!(client.get_alert_count(), 1);
}

#[test]
fn returns_first_error_when_several_inputs_are_bad() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    // Bad hash AND bad rule: the hash check runs first.
    let result = client.try_validate_alert_input(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, "short"),
        &vec![&env, s(&env, "rule:bogus")],
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}
