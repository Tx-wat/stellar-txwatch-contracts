//! Tests for `get_alert_summary` (#419).
//!
//! `total` must equal the global monotonic counter; `active` and `inactive`
//! must count only the owner's live alerts, split by their `active` flag.

use crate::{AlertRegistry, AlertRegistryClient, AlertSummary};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

fn setup() -> (Env, AlertRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn register(env: &Env, client: &AlertRegistryClient<'_>, owner: &Address) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "alert"),
        &String::from_str(
            env,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn summary_for_owner_with_no_alerts_is_all_zero() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);

    assert_eq!(
        client.get_alert_summary(&owner),
        AlertSummary {
            total: 0,
            active: 0,
            inactive: 0,
        }
    );
}

#[test]
fn summary_counts_the_owners_live_alerts_and_the_global_total() {
    let (env, client, _admin) = setup();
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    register(&env, &client, &owner_a);
    register(&env, &client, &owner_a);
    register(&env, &client, &owner_b);

    // total is the global counter, so it includes owner_b's alert too.
    let summary_a = client.get_alert_summary(&owner_a);
    assert_eq!(summary_a.total, client.get_alert_count());
    assert_eq!(summary_a.total, 3);
    assert_eq!(summary_a.active, 2);
    assert_eq!(summary_a.inactive, 0);

    let summary_b = client.get_alert_summary(&owner_b);
    assert_eq!(summary_b.total, 3);
    assert_eq!(summary_b.active, 1);
    assert_eq!(summary_b.inactive, 0);
}

#[test]
fn paused_alerts_are_counted_as_inactive() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let first = register(&env, &client, &owner);
    register(&env, &client, &owner);
    register(&env, &client, &owner);

    client.pause_alert(&admin, &first);

    let summary = client.get_alert_summary(&owner);
    assert_eq!(summary.active, 2);
    assert_eq!(summary.inactive, 1);
    assert_eq!(summary.total, 3);
}

#[test]
fn removed_alerts_leave_the_live_counts_but_not_the_total() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let first = register(&env, &client, &owner);
    register(&env, &client, &owner);

    client.remove_alert(&owner, &first);

    let summary = client.get_alert_summary(&owner);
    assert_eq!(summary.total, 2, "total is monotonic and never decremented");
    assert_eq!(summary.active, 1);
    assert_eq!(summary.inactive, 0);
    assert_eq!(summary.active, client.get_active_alert_count(&owner));
}

#[test]
fn summary_active_plus_inactive_matches_get_active_alert_count() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let a = register(&env, &client, &owner);
    register(&env, &client, &owner);
    register(&env, &client, &owner);
    client.pause_alert(&admin, &a);

    let summary = client.get_alert_summary(&owner);
    assert_eq!(
        summary.active + summary.inactive,
        client.get_active_alert_count(&owner)
    );
}
