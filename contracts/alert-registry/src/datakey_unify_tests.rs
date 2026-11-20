//! Tests for the single `DataKey` definition (#429).
//!
//! `lib.rs` and `types.rs` must share one `DataKey` enum. The compile-time
//! assignments below fail to build if the two declarations diverge again.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::types::DataKey as TypesDataKey;
use crate::{AlertRegistry, AlertRegistryClient, DataKey as LibDataKey};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

#[test]
fn lib_and_types_datakey_are_the_same_type() {
    // Only compiles if `crate::DataKey` is `crate::types::DataKey`.
    let key: TypesDataKey = LibDataKey::NextId;
    assert_eq!(key, TypesDataKey::NextId);
}

#[test]
fn alert_storage_is_addressed_by_the_shared_key() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "keyed"),
        &String::from_str(&env, "a3f1b2c4d5e6f7089abcdef01234567890abcdef01234567890abcdef012345"),
        &vec![&env, String::from_str(&env, "rule:mint")],
        &None,
    );

    let present = env.as_contract(&client.address, || {
        env.storage().persistent().has(&TypesDataKey::Alert(id))
    });
    assert!(present);
}

#[test]
fn owner_index_is_addressed_by_the_shared_key() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "owner index"),
        &String::from_str(&env, "a3f1b2c4d5e6f7089abcdef01234567890abcdef01234567890abcdef012345"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
        &None,
    );

    let indexed = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get::<_, soroban_sdk::Vec<u64>>(&TypesDataKey::OwnerIndex(owner.clone()))
            .map(|ids| ids.contains(id))
            .unwrap_or(false)
    });
    assert!(indexed);
}
