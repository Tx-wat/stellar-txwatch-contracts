//! Tests for `is_watcher_authorized` across many watchers, removals, and expiry (#485).
//!
//! The check currently reads the full watcher list on every call. These tests pin the result
//! for each watcher so the lookup can change to a keyed map without altering behavior.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, Vec};

use crate::{WatcherRegistry, WatcherRegistryClient};

const WATCHERS: u32 = 100;

fn setup() -> (Env, WatcherRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WatcherRegistry, ());
    let client = WatcherRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn every_registered_watcher_is_authorized() {
    let (env, client, admin) = setup();
    let mut watchers: Vec<Address> = Vec::new(&env);
    for _ in 0..WATCHERS {
        let w = Address::generate(&env);
        client.register_watcher(&admin, &w, &None);
        watchers.push_back(w);
    }
    for w in watchers.iter() {
        assert!(client.is_watcher_authorized(&w));
    }
}

#[test]
fn unknown_address_is_not_authorized() {
    let (env, client, admin) = setup();
    for _ in 0..WATCHERS {
        client.register_watcher(&admin, &Address::generate(&env), &None);
    }
    assert!(!client.is_watcher_authorized(&Address::generate(&env)));
}

#[test]
fn removing_one_watcher_does_not_affect_the_others() {
    let (env, client, admin) = setup();
    let mut watchers: Vec<Address> = Vec::new(&env);
    for _ in 0..WATCHERS {
        let w = Address::generate(&env);
        client.register_watcher(&admin, &w, &None);
        watchers.push_back(w);
    }

    let removed = watchers.get(WATCHERS / 2).unwrap();
    client.remove_watcher(&admin, &removed);

    assert!(!client.is_watcher_authorized(&removed));
    for w in watchers.iter() {
        if w != removed {
            assert!(client.is_watcher_authorized(&w));
        }
    }
}

#[test]
fn expired_watcher_is_rejected_and_live_one_is_kept() {
    let (env, client, admin) = setup();
    env.ledger().set_timestamp(1_000);

    let expiring = Address::generate(&env);
    let lasting = Address::generate(&env);
    client.register_watcher(&admin, &expiring, &Some(2_000));
    client.register_watcher(&admin, &lasting, &None);

    assert!(client.is_watcher_authorized(&expiring));

    env.ledger().set_timestamp(2_001);
    assert!(!client.is_watcher_authorized(&expiring));
    assert!(client.is_watcher_authorized(&lasting));
}
