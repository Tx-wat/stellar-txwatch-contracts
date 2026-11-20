//! Owner-to-delegate TTL renewal (#425).
//!
//! A delegate may renew an alert's TTL on the owner's behalf. It must not be
//! able to transfer, update or remove the alert, and clearing the delegate
//! revokes access immediately.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient, ContractError};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client)
}

fn register(env: &Env, client: &AlertRegistryClient<'static>, owner: &Address) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "alert"),
        &String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn owner_can_set_a_delegate() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);

    client.set_delegate(&owner, &id, &Some(delegate.clone()));
    assert_eq!(client.get_delegate(&id), Some(delegate));
}

#[test]
fn non_owner_cannot_set_a_delegate() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = register(&env, &client, &owner);

    let result = client.try_set_delegate(&stranger, &id, &Some(stranger.clone()));
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_delegate(&id), None);
}

#[test]
fn delegate_can_renew_ttl() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate.clone()));

    assert_eq!(client.try_renew_alert_ttl(&delegate, &id), Ok(Ok(())));
}

#[test]
fn owner_can_still_renew_ttl_without_a_delegate() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let id = register(&env, &client, &owner);

    assert_eq!(client.try_renew_alert_ttl(&owner, &id), Ok(Ok(())));
}

#[test]
fn stranger_cannot_renew_ttl() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = register(&env, &client, &owner);

    let result = client.try_renew_alert_ttl(&stranger, &id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn delegate_is_scoped_to_one_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id_a = register(&env, &client, &owner);
    let id_b = register(&env, &client, &owner);
    client.set_delegate(&owner, &id_a, &Some(delegate.clone()));

    assert_eq!(client.try_renew_alert_ttl(&delegate, &id_a), Ok(Ok(())));
    let result = client.try_renew_alert_ttl(&delegate, &id_b);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn delegate_cannot_update_or_remove_the_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate.clone()));

    let update = client.try_update_alert(
        &delegate,
        &id,
        &vec![&env, String::from_str(&env, "rule:transfer")],
        &true,
    );
    assert_eq!(update, Err(Ok(ContractError::Unauthorized)));

    let remove = client.try_remove_alert(&delegate, &id);
    assert_eq!(remove, Err(Ok(ContractError::Unauthorized)));
    assert!(client.get_alert(&id).is_some());
}

#[test]
fn delegate_cannot_reassign_itself_or_another_delegate() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate.clone()));

    let result = client.try_set_delegate(&delegate, &id, &Some(delegate.clone()));
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn clearing_the_delegate_revokes_access_immediately() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate.clone()));

    client.set_delegate(&owner, &id, &None);
    assert_eq!(client.get_delegate(&id), None);
    let result = client.try_renew_alert_ttl(&delegate, &id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn set_delegate_on_missing_alert_is_not_found() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);

    let result = client.try_set_delegate(&owner, &999, &Some(delegate));
    assert_eq!(result, Err(Ok(ContractError::AlertNotFound)));
}

#[test]
fn removing_the_alert_clears_its_delegate() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate));

    client.remove_alert(&owner, &id);
    assert_eq!(client.get_delegate(&id), None);
}

#[test]
fn renew_emits_alert_bump_event() {
    use soroban_sdk::testutils::Events as _;
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let id = register(&env, &client, &owner);
    client.set_delegate(&owner, &id, &Some(delegate.clone()));

    client.renew_alert_ttl(&delegate, &id);
    assert!(!env.events().all().is_empty(), "renew_alert_ttl must emit an event");
}
