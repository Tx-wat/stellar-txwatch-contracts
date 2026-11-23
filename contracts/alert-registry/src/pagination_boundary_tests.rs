//! Boundary cases for the paginated alert queries (#265).
//!
//! Five alerts are registered for one target contract, labelled `a0`..`a4` in ID order, so each
//! page can be checked by content, not only by length. `limit = MAX_BATCH_SIZE + 1` is not covered:
//! the compiled `configs_paginated` does not clamp `limit`, and the issue's expected behaviour is
//! not decided yet.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup() -> (Env, AlertRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    for i in 0..5 {
        let label = String::from_str(&env, &format!("a{i}"));
        client.register_alert(
            &owner,
            &target,
            &label,
            &String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            &vec![&env, String::from_str(&env, "rule:transfer")],
        );
    }
    (env, client, target)
}

#[test]
fn empty_index_returns_an_empty_page() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let target = Address::generate(&env);
    let page = client.try_get_contract_alerts_paginated(&target, &target, &0, &10).unwrap().unwrap();
    assert_eq!(page.len(), 0);
}

#[test]
fn first_and_second_pages_hold_the_expected_alerts() {
    let (env, client, target) = setup();

    let first = client.try_get_contract_alerts_paginated(&target, &target, &0, &3).unwrap().unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first.get(0).unwrap().label, String::from_str(&env, "a0"));
    assert_eq!(first.get(2).unwrap().label, String::from_str(&env, "a2"));

    let second = client.try_get_contract_alerts_paginated(&target, &target, &3, &3).unwrap().unwrap();
    assert_eq!(second.len(), 2);
    assert_eq!(second.get(0).unwrap().label, String::from_str(&env, "a3"));
    assert_eq!(second.get(1).unwrap().label, String::from_str(&env, "a4"));
}

#[test]
fn offset_past_the_end_returns_an_empty_page() {
    let (_env, client, target) = setup();
    let page = client.try_get_contract_alerts_paginated(&target, &target, &100, &10).unwrap().unwrap();
    assert_eq!(page.len(), 0);
}

#[test]
fn zero_limit_returns_an_empty_page() {
    let (_env, client, target) = setup();
    let page = client.try_get_contract_alerts_paginated(&target, &target, &0, &0).unwrap().unwrap();
    assert_eq!(page.len(), 0);
}

#[test]
fn offset_equal_to_length_returns_an_empty_page() {
    let (_env, client, target) = setup();
    let page = client.try_get_contract_alerts_paginated(&target, &target, &5, &5).unwrap().unwrap();
    assert_eq!(page.len(), 0);
}

#[test]
fn offset_near_u32_max_does_not_overflow() {
    let (_env, client, target) = setup();
    let page = client.try_get_contract_alerts_paginated(&target, &target, &u32::MAX, &u32::MAX).unwrap().unwrap();
    assert_eq!(page.len(), 0);
}
