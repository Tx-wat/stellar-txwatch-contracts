//! Asserts the topics of the events the compiled `AlertRegistry` emits (#267).
//!
//! If a topic is renamed, these tests fail, so off-chain indexers see the break in CI rather than
//! in production. Only events present in the compiled `lib.rs` are covered.

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Events as _}, vec, Address, Env, String, Symbol, TryFromVal};

use crate::{AlertRegistry, AlertRegistryClient};

/// True when some emitted event has exactly the two topics `(a, b)`.
fn emitted(env: &Env, a: &str, b: &str) -> bool {
    let want_a = Symbol::new(env, a);
    let want_b = Symbol::new(env, b);
    env.events().all().iter().any(|(_contract, topics, _data)| {
        topics.len() == 2
            && Symbol::try_from_val(env, &topics.get(0).unwrap()).ok() == Some(want_a.clone())
            && Symbol::try_from_val(env, &topics.get(1).unwrap()).ok() == Some(want_b.clone())
    })
}

fn setup() -> (Env, AlertRegistryClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    (env, client, owner, target)
}

#[test]
fn register_emits_alert_register() {
    let (env, client, owner, target) = setup();
    client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "alert"),
        &String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
    );
    assert!(emitted(&env, "alert", "register"));
}

#[test]
fn remove_emits_alert_remove() {
    let (env, client, owner, target) = setup();
    let id = client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "alert"),
        &String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
    );
    client.remove_alert(&owner, &id);
    assert!(emitted(&env, "alert", "remove"));
}

#[test]
fn bump_emits_alert_bump() {
    let (env, client, owner, target) = setup();
    let id = client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "alert"),
        &String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
    );
    client.bump_alert(&id, &1_000);
    assert!(emitted(&env, "alert", "bump"));
}
