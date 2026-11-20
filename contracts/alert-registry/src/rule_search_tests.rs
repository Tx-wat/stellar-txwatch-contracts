//! `get_alerts_by_rule` returns only active alerts that contain the rule (#416).

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

use crate::{AlertRegistry, AlertRegistryClient, MAX_BATCH_SIZE};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn register_with_rules(
    env: &Env,
    client: &AlertRegistryClient<'static>,
    owner: &Address,
    rules: &[&str],
) -> u64 {
    let mut v: Vec<String> = Vec::new(env);
    for r in rules {
        v.push_back(String::from_str(env, r));
    }
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "alert"),
        &String::from_str(env, HASH),
        &v,
        &None,
    )
}

#[test]
fn returns_empty_when_no_alerts_exist() {
    let (env, client) = setup();
    let rule = String::from_str(&env, "rule:transfer");
    assert_eq!(client.get_alerts_by_rule(&rule).len(), 0);
}

#[test]
fn returns_only_alerts_containing_the_rule() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    register_with_rules(&env, &client, &owner, &["rule:transfer"]);
    register_with_rules(&env, &client, &owner, &["rule:mint"]);
    register_with_rules(&env, &client, &owner, &["rule:transfer", "rule:mint"]);

    let transfer = String::from_str(&env, "rule:transfer");
    let found = client.get_alerts_by_rule(&transfer);
    assert_eq!(found.len(), 2);
    for cfg in found.iter() {
        assert!(cfg.rules.contains(&transfer));
    }
}

#[test]
fn skips_inactive_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let id = register_with_rules(&env, &client, &owner, &["rule:transfer"]);
    register_with_rules(&env, &client, &owner, &["rule:transfer"]);
    client.deactivate_all_alerts(&owner);

    let transfer = String::from_str(&env, "rule:transfer");
    assert_eq!(client.get_alerts_by_rule(&transfer).len(), 0);

    client.update_alert(&owner, &id, &vec![&env, transfer.clone()], &true);
    let found = client.get_alerts_by_rule(&transfer);
    assert_eq!(found.len(), 1);
    assert!(found.get(0).unwrap().active);
}

#[test]
fn skips_removed_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    register_with_rules(&env, &client, &owner, &["rule:invoke"]);
    let removed = register_with_rules(&env, &client, &owner, &["rule:invoke"]);
    client.remove_alert(&owner, &removed);

    let invoke = String::from_str(&env, "rule:invoke");
    assert_eq!(client.get_alerts_by_rule(&invoke).len(), 1);
}

#[test]
fn unknown_rule_text_matches_nothing() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    register_with_rules(&env, &client, &owner, &["rule:all"]);

    let other = String::from_str(&env, "rule:mint");
    assert_eq!(client.get_alerts_by_rule(&other).len(), 0);
}

#[test]
fn result_is_capped_at_max_batch_size() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    for _ in 0..(MAX_BATCH_SIZE + 3) {
        register_with_rules(&env, &client, &owner, &["rule:all"]);
    }
    let all = String::from_str(&env, "rule:all");
    assert_eq!(client.get_alerts_by_rule(&all).len(), MAX_BATCH_SIZE);
}
