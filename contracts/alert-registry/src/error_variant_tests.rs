//! One test per `ContractError` variant that the compiled `AlertRegistry` can return (#259).

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient, ContractError};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn s(env: &Env, text: &str) -> String {
    String::from_str(env, text)
}

fn hash64(env: &Env) -> String {
    String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

#[test]
fn already_initialized_on_second_initialize() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let again = client.try_initialize(&admin);
    assert_eq!(again, Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
fn not_initialized_before_initialize() {
    let (_env, client) = setup();
    let result = client.try_get_admin();
    assert_eq!(result, Err(Ok(ContractError::NotInitialized)));
}

#[test]
fn unauthorized_when_a_stranger_removes_an_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &hash64(&env),
        &vec![&env, s(&env, "rule:transfer")],
    );
    let result = client.try_remove_alert(&stranger, &id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn alert_not_found_when_bumping_a_missing_id() {
    let (_env, client) = setup();
    let result = client.try_bump_alert(&999, &1_000);
    assert_eq!(result, Err(Ok(ContractError::AlertNotFound)));
}

#[test]
fn invalid_rule_descriptor_for_an_unknown_rule() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &hash64(&env),
        &vec![&env, s(&env, "rule:bogus")],
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidRuleDescriptor)));
}

#[test]
fn invalid_webhook_hash_when_hash_too_short() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client.try_register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &s(&env, "tooshort"),
        &vec![&env, s(&env, "rule:transfer")],
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

#[test]
fn invalid_webhook_hash_on_update_webhook() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &hash64(&env),
        &vec![&env, s(&env, "rule:transfer")],
    );
    let result = client.try_update_webhook(&owner, &id, &s(&env, "tooshort"));
    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

#[test]
fn invalid_webhook_hash_on_propose_webhook() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &hash64(&env),
        &vec![&env, s(&env, "rule:transfer")],
    );
    let result = client.try_propose_webhook(&owner, &id, &s(&env, "tooshort"));
    assert_eq!(result, Err(Ok(ContractError::InvalidWebhookHash)));
}

#[test]
fn no_pending_webhook_when_confirm_called_without_propose() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &s(&env, "alert"),
        &hash64(&env),
        &vec![&env, s(&env, "rule:transfer")],
    );
    let result = client.try_confirm_webhook(&owner, &id);
    assert_eq!(result, Err(Ok(ContractError::NoPendingWebhook)));
}

#[test]
fn owner_alert_limit_exceeded_on_the_second_registration() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.set_per_owner_alert_limit(&admin, &1);

    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let rules = vec![&env, s(&env, "rule:transfer")];
    client.register_alert(&owner, &target, &s(&env, "first"), &hash64(&env), &rules);

    let result = client.try_register_alert(&owner, &target, &s(&env, "second"), &hash64(&env), &rules);
    assert_eq!(result, Err(Ok(ContractError::OwnerAlertLimitExceeded)));
}
