//! Tests for per-alert tags in `batch_register_alerts` (#428).
//!
//! `AlertInput` gains an optional `tags` field. Each batch entry is validated
//! with the same limits as `register_alert` and stored on its own `AlertConfig`.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

use crate::{AlertInput, AlertRegistry, AlertRegistryClient, ContractError, MAX_TAGS};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn hash64(env: &Env) -> String {
    String::from_str(env, "a3f1b2c4d5e6f7089abcdef01234567890abcdef01234567890abcdef012345")
}

fn input(env: &Env, target: &Address, tags: Option<Vec<String>>) -> AlertInput {
    AlertInput {
        target_contract: target.clone(),
        label: String::from_str(env, "batch"),
        webhook_hash: hash64(env),
        rules: vec![env, String::from_str(env, "rule:transfer")],
        tags,
    }
}

#[test]
fn batch_stores_tags_per_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let first_tags = vec![&env, String::from_str(&env, "alpha")];
    let second_tags = vec![
        &env,
        String::from_str(&env, "beta"),
        String::from_str(&env, "gamma"),
    ];
    let ids = client.batch_register_alerts(
        &owner,
        &vec![
            &env,
            input(&env, &target, Some(first_tags.clone())),
            input(&env, &target, Some(second_tags.clone())),
        ],
    );

    assert_eq!(client.get_alert(&ids.get(0).unwrap()).unwrap().tags, Some(first_tags));
    assert_eq!(client.get_alert(&ids.get(1).unwrap()).unwrap().tags, Some(second_tags));
}

#[test]
fn batch_without_tags_stores_none() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let ids = client.batch_register_alerts(&owner, &vec![&env, input(&env, &target, None)]);

    assert_eq!(client.get_alert(&ids.get(0).unwrap()).unwrap().tags, None);
}

#[test]
fn batch_rejects_too_many_tags_on_one_entry() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let mut tags: Vec<String> = vec![&env];
    for _ in 0..=MAX_TAGS {
        tags.push_back(String::from_str(&env, "t"));
    }
    let res = client.try_batch_register_alerts(
        &owner,
        &vec![&env, input(&env, &target, Some(tags))],
    );

    assert_eq!(res, Err(Ok(ContractError::TooManyTags)));
}

#[test]
fn batch_rejects_whole_batch_when_one_entry_has_bad_tags() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let mut too_many: Vec<String> = vec![&env];
    for _ in 0..=MAX_TAGS {
        too_many.push_back(String::from_str(&env, "t"));
    }
    let res = client.try_batch_register_alerts(
        &owner,
        &vec![
            &env,
            input(&env, &target, None),
            input(&env, &target, Some(too_many)),
        ],
    );

    assert_eq!(res, Err(Ok(ContractError::TooManyTags)));
    // Validation runs before any write, so the valid first entry was not stored.
    assert_eq!(client.get_alert_count(), 0);
}
