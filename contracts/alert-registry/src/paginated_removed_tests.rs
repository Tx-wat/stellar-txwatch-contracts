//! Tests for `get_alerts_by_owner_paginated` after removals (#486).
//!
//! Removing an alert also removes its ID from the owner index, so `offset` and `limit` address
//! positions among the *live* alerts only. These tests pin that behavior: pages never contain
//! gaps, and a page past the live entries is empty.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup() -> (
    Env,
    AlertRegistryClient<'static>,
    Address,
    Address,
    [u64; 6],
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);

    let querier = Address::generate(&env);
    let owner = Address::generate(&env);
    let mut ids = [0u64; 6];
    for (i, slot) in ids.iter_mut().enumerate() {
        let target = Address::generate(&env);
        let label = String::from_str(&env, ["a0", "a1", "a2", "a3", "a4", "a5"][i]);
        *slot = client.register_alert(
            &owner,
            &target,
            &label,
            &String::from_str(
                &env,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            &vec![&env, String::from_str(&env, "rule:transfer")],
            &None,
        );
    }
    (env, client, querier, owner, ids)
}

#[test]
fn page_after_removal_has_no_gap() {
    let (env, client, querier, owner, ids) = setup();
    client.remove_alert(&owner, &ids[1]);

    let page = client.get_alerts_by_owner_paginated(&querier, &owner, &0, &3);
    assert_eq!(page.len(), 3);
    assert_eq!(page.get(0).unwrap().label, String::from_str(&env, "a0"));
    assert_eq!(page.get(1).unwrap().label, String::from_str(&env, "a2"));
    assert_eq!(page.get(2).unwrap().label, String::from_str(&env, "a3"));
}

#[test]
fn offset_counts_only_live_entries() {
    let (env, client, querier, owner, ids) = setup();
    client.remove_alert(&owner, &ids[0]);
    client.remove_alert(&owner, &ids[1]);

    // Live owner index is now [a2, a3, a4, a5].
    let page = client.get_alerts_by_owner_paginated(&querier, &owner, &2, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().label, String::from_str(&env, "a4"));
    assert_eq!(page.get(1).unwrap().label, String::from_str(&env, "a5"));
}

#[test]
fn offset_at_live_length_returns_empty_page() {
    let (_env, client, querier, owner, ids) = setup();
    client.remove_alert(&owner, &ids[2]);
    client.remove_alert(&owner, &ids[3]);

    // Four live alerts remain, so offset 4 is exactly past the end.
    let page = client.get_alerts_by_owner_paginated(&querier, &owner, &4, &2);
    assert_eq!(page.len(), 0);
}

#[test]
fn offset_past_end_with_removed_entries_is_empty() {
    let (_env, client, querier, owner, ids) = setup();
    client.remove_alert(&owner, &ids[5]);

    let page = client.get_alerts_by_owner_paginated(&querier, &owner, &6, &10);
    assert_eq!(page.len(), 0);
}
