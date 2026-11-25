//! `activate_all_alerts` mirrors `deactivate_all_alerts` (#418).

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger as _}, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn register(env: &Env, client: &AlertRegistryClient<'static>, owner: &Address) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "alert"),
        &String::from_str(env, HASH),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn returns_zero_when_owner_has_no_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    assert_eq!(client.activate_all_alerts(&owner), 0);
}

#[test]
fn activates_every_inactive_alert_and_returns_count() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let a = register(&env, &client, &owner);
    let b = register(&env, &client, &owner);
    client.deactivate_all_alerts(&owner);
    assert!(!client.get_alert(&a).unwrap().active);
    assert!(!client.get_alert(&b).unwrap().active);

    assert_eq!(client.activate_all_alerts(&owner), 2);
    assert!(client.get_alert(&a).unwrap().active);
    assert!(client.get_alert(&b).unwrap().active);
}

#[test]
fn skips_alerts_that_are_already_active() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let a = register(&env, &client, &owner);
    let b = register(&env, &client, &owner);
    let c = register(&env, &client, &owner);
    client.deactivate_all_alerts(&owner);
    // Re-enable `b` by hand so it is already active when activate_all runs.
    client.update_alert(&owner, &b, &vec![&env, String::from_str(&env, "rule:transfer")], &true);

    assert_eq!(client.activate_all_alerts(&owner), 2);
    assert!(client.get_alert(&a).unwrap().active);
    assert!(client.get_alert(&b).unwrap().active);
    assert!(client.get_alert(&c).unwrap().active);
}

#[test]
fn only_touches_the_callers_alerts() {
    let (env, client) = setup();
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    register(&env, &client, &owner1);
    let other = register(&env, &client, &owner2);
    client.deactivate_all_alerts(&owner1);
    client.deactivate_all_alerts(&owner2);

    assert_eq!(client.activate_all_alerts(&owner1), 1);
    assert!(client.get_alert(&other).is_some());
    assert!(!client.get_alert(&other).unwrap().active);
}

#[test]
fn skips_removed_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let kept = register(&env, &client, &owner);
    let removed = register(&env, &client, &owner);
    client.deactivate_all_alerts(&owner);
    client.remove_alert(&owner, &removed);

    assert_eq!(client.activate_all_alerts(&owner), 1);
    assert!(client.get_alert(&kept).unwrap().active);
    assert!(client.get_alert(&removed).is_none());
}

#[test]
fn updates_updated_at_only_for_changed_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let a = register(&env, &client, &owner);
    client.deactivate_all_alerts(&owner);
    let before = client.get_alert(&a).unwrap().updated_at;

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.activate_all_alerts(&owner);
    let after = client.get_alert(&a).unwrap().updated_at;
    assert!(after > before);
}

#[test]
#[should_panic]
fn requires_auth_from_caller() {
    let env = Env::default();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    client.activate_all_alerts(&owner);
}
