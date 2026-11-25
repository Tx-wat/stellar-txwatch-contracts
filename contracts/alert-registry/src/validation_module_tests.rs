//! Behavioural tests for the alert validation helpers (#430).
//!
//! These go through the public entry points so they hold before and after the
//! helpers move into `validation.rs`. Each boundary is checked on both sides.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

use crate::{AlertRegistry, AlertRegistryClient, ContractError, MAX_TAGS, MAX_TAG_BYTES};

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

fn repeat(env: &Env, ch: u8, n: usize) -> String {
    let buf = [ch; 128];
    String::from_bytes(env, &buf[..n])
}

fn one_rule(env: &Env) -> Vec<String> {
    vec![env, String::from_str(env, "rule:transfer")]
}

#[test]
fn label_at_128_bytes_is_accepted_and_129_is_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let ok = client.try_register_alert(&owner, &target, &repeat(&env, b'a', 128), &hash64(&env), &one_rule(&env), &None);
    assert!(ok.is_ok());

    // 129 bytes is a single over-limit label: the boundary the validator must enforce.
    let long = String::from_bytes(&env, &[b'a'; 129]);
    let res = client.try_register_alert(&owner, &target, &long, &hash64(&env), &one_rule(&env), &None);
    assert_eq!(res, Err(Ok(ContractError::LabelTooLong)));
}

#[test]
fn empty_rules_are_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let res = client.try_register_alert(
        &owner,
        &target,
        &String::from_str(&env, "x"),
        &hash64(&env),
        &vec![&env],
        &None,
    );
    assert_eq!(res, Err(Ok(ContractError::EmptyRules)));
}

#[test]
fn duplicate_rules_are_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let res = client.try_register_alert(
        &owner,
        &target,
        &String::from_str(&env, "x"),
        &hash64(&env),
        &vec![
            &env,
            String::from_str(&env, "rule:mint"),
            String::from_str(&env, "rule:mint"),
        ],
        &None,
    );
    assert_eq!(res, Err(Ok(ContractError::DuplicateRule)));
}

#[test]
fn unknown_rule_is_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let res = client.try_register_alert(
        &owner,
        &target,
        &String::from_str(&env, "x"),
        &hash64(&env),
        &vec![&env, String::from_str(&env, "rule:bogus")],
        &None,
    );
    assert_eq!(res, Err(Ok(ContractError::InvalidRuleDescriptor)));
}

#[test]
fn tag_count_at_limit_is_accepted_and_one_over_is_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let mut at_limit: Vec<String> = vec![&env];
    for _ in 0..MAX_TAGS {
        at_limit.push_back(String::from_str(&env, "t"));
    }
    assert!(client
        .try_register_alert(&owner, &target, &String::from_str(&env, "x"), &hash64(&env), &one_rule(&env), &Some(at_limit.clone()))
        .is_ok());

    at_limit.push_back(String::from_str(&env, "t"));
    let res = client.try_register_alert(&owner, &target, &String::from_str(&env, "x"), &hash64(&env), &one_rule(&env), &Some(at_limit));
    assert_eq!(res, Err(Ok(ContractError::TooManyTags)));
}

#[test]
fn tag_length_at_limit_is_accepted_and_one_over_is_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let max_len = String::from_bytes(&env, &[b'g'; MAX_TAG_BYTES as usize]);
    assert!(client
        .try_register_alert(&owner, &target, &String::from_str(&env, "x"), &hash64(&env), &one_rule(&env), &Some(vec![&env, max_len]))
        .is_ok());

    let too_long = String::from_bytes(&env, &[b'g'; MAX_TAG_BYTES as usize + 1]);
    let res = client.try_register_alert(&owner, &target, &String::from_str(&env, "x"), &hash64(&env), &one_rule(&env), &Some(vec![&env, too_long]));
    assert_eq!(res, Err(Ok(ContractError::TagTooLong)));
}

#[test]
fn invalid_webhook_hash_length_is_rejected() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let res = client.try_register_alert(
        &owner,
        &target,
        &String::from_str(&env, "x"),
        &String::from_str(&env, "abc"),
        &one_rule(&env),
        &None,
    );
    assert_eq!(res, Err(Ok(ContractError::InvalidWebhookHash)));
}
