//! Tests for `get_alert_ids_by_owner_paginated` (#413): pages address live IDs,
//! the limit is capped at `MAX_BATCH_SIZE`, and removed alerts never appear.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

use crate::{AlertRegistry, AlertRegistryClient, MAX_BATCH_SIZE};

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
        &String::from_str(env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn pages_concatenate_to_the_full_list() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    for _ in 0..5 {
        register(&env, &client, &owner);
    }
    let all = client.get_alert_ids_by_owner(&owner);

    let p0 = client.get_alert_ids_by_owner_paginated(&owner, &0, &2);
    let p1 = client.get_alert_ids_by_owner_paginated(&owner, &2, &2);
    let p2 = client.get_alert_ids_by_owner_paginated(&owner, &4, &2);
    assert_eq!(p0, all.slice(0..2));
    assert_eq!(p1, all.slice(2..4));
    assert_eq!(p2, all.slice(4..5));
}

#[test]
fn offset_at_or_past_the_end_is_empty() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    register(&env, &client, &owner);
    register(&env, &client, &owner);

    assert_eq!(client.get_alert_ids_by_owner_paginated(&owner, &2, &5).len(), 0);
    assert_eq!(client.get_alert_ids_by_owner_paginated(&owner, &u32::MAX, &5).len(), 0);
}

#[test]
fn limit_is_capped_at_max_batch_size() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    for _ in 0..(MAX_BATCH_SIZE + 3) {
        register(&env, &client, &owner);
    }
    let page = client.get_alert_ids_by_owner_paginated(&owner, &0, &u32::MAX);
    assert_eq!(page.len(), MAX_BATCH_SIZE);
    assert_eq!(page, client.get_alert_ids_by_owner(&owner).slice(0..MAX_BATCH_SIZE));
}

#[test]
fn removed_alerts_are_skipped_and_do_not_take_a_slot() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let ids: Vec<u64> = {
        let mut v = Vec::new(&env);
        for _ in 0..4 {
            v.push_back(register(&env, &client, &owner));
        }
        v
    };
    client.remove_alert(&owner, &ids.get(1).unwrap());

    let live = client.get_alert_ids_by_owner(&owner);
    assert_eq!(live.len(), 3);
    assert!(!live.contains(&ids.get(1).unwrap()));

    // The first page of two live IDs is the first two survivors, not the
    // first two registered IDs.
    let page = client.get_alert_ids_by_owner_paginated(&owner, &0, &2);
    assert_eq!(page, live.slice(0..2));
    assert!(!page.contains(&ids.get(1).unwrap()));
    assert_eq!(client.get_alert_ids_by_owner_paginated(&owner, &2, &2), live.slice(2..3));
}

#[test]
fn only_returns_ids_owned_by_the_requested_owner() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    let mine = register(&env, &client, &owner);
    register(&env, &client, &other);

    let page = client.get_alert_ids_by_owner_paginated(&owner, &0, &10);
    assert_eq!(page, vec![&env, mine]);
}
