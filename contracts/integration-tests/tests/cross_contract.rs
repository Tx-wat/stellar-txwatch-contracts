use alert_registry::{AlertRegistry, AlertRegistryClient, ContractError as AlertError};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};
use watcher_registry::{WatcherRegistry, WatcherRegistryClient};

fn setup() -> (
    Env,
    AlertRegistryClient<'static>,
    WatcherRegistryClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let alert_id = env.register_contract(None, AlertRegistry);
    let watcher_id = env.register_contract(None, WatcherRegistry);

    let alert_client = AlertRegistryClient::new(&env, &alert_id);
    let watcher_client = WatcherRegistryClient::new(&env, &watcher_id);

    (env, alert_client, watcher_client)
}

/// An authorized watcher can query AlertRegistry and see registered alerts
/// when no watcher-gating is configured (open access mode).
#[test]
fn test_authorized_watcher_can_query_alert_registry_open_mode() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let watcher = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    // Initialize watcher registry and authorize the watcher
    watcher_client.initialize(&admin);
    watcher_client.register_watcher(&admin, &watcher);

    // Register an alert in the alert registry
    let id = alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Cross-contract alert"),
        &String::from_str(&env, "webhook-hash-abc"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
    );

    // Verify the watcher is authorized in the watcher registry
    assert!(watcher_client.is_watcher_authorized(&watcher));

    // No gating configured — watcher queries the alert registry freely
    let alerts = alert_client
        .get_alerts_for_contract(&watcher, &target)
        .unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        alerts.get(0).unwrap().label,
        String::from_str(&env, "Cross-contract alert")
    );

    let cfg = alert_client.get_alert(&id).unwrap();
    assert_eq!(cfg.owner, owner);
    assert_eq!(cfg.target_contract, target);
    assert!(cfg.active);
}

/// An unauthorized address is not a watcher and cannot be confused with one.
#[test]
fn test_unauthorized_address_not_a_watcher() {
    let (env, _alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);

    watcher_client.initialize(&admin);

    assert!(!watcher_client.is_watcher_authorized(&stranger));
}

/// Removing a watcher revokes their authorization while alert data is unaffected.
#[test]
fn test_removed_watcher_loses_authorization_alert_data_intact() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let watcher = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    watcher_client.initialize(&admin);
    watcher_client.register_watcher(&admin, &watcher);

    alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Alert"),
        &String::from_str(&env, "hash"),
        &vec![&env],
    );

    // Remove the watcher
    watcher_client.remove_watcher(&admin, &watcher);
    assert!(!watcher_client.is_watcher_authorized(&watcher));

    // Alert data is still intact (no gating configured)
    assert_eq!(
        alert_client
            .get_alerts_for_contract(&watcher, &target)
            .unwrap()
            .len(),
        1
    );
}

/// When watcher-gating is enabled, a registered watcher can read alert data.
#[test]
fn test_watcher_gating_registered_watcher_can_read() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let watcher = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    // Set up watcher registry
    watcher_client.initialize(&admin);
    watcher_client.register_watcher(&admin, &watcher);

    // Initialize alert registry and point it at the watcher registry
    alert_client.initialize(&admin).unwrap();
    let watcher_contract_id = watcher_client.address.clone();
    alert_client
        .set_watcher_registry(&admin, &watcher_contract_id)
        .unwrap();

    alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Gated Alert"),
        &String::from_str(&env, "hash"),
        &vec![&env, String::from_str(&env, "rule:transfer")],
    );

    // Registered watcher can read
    let results = alert_client
        .get_alerts_for_contract(&watcher, &target)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results.get(0).unwrap().label,
        String::from_str(&env, "Gated Alert")
    );
}

/// When watcher-gating is enabled, an unregistered address is rejected.
#[test]
fn test_watcher_gating_unregistered_address_rejected() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    watcher_client.initialize(&admin);
    // stranger is NOT registered as a watcher

    alert_client.initialize(&admin).unwrap();
    let watcher_contract_id = watcher_client.address.clone();
    alert_client
        .set_watcher_registry(&admin, &watcher_contract_id)
        .unwrap();

    alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Alert"),
        &String::from_str(&env, "hash"),
        &vec![&env],
    );

    assert_eq!(
        alert_client
            .try_get_alerts_for_contract(&stranger, &target)
            .unwrap_err()
            .unwrap(),
        AlertError::NotAWatcher
    );
}

/// When watcher-gating is enabled, a removed watcher loses read access.
#[test]
fn test_watcher_gating_removed_watcher_loses_access() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let watcher = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    watcher_client.initialize(&admin);
    watcher_client.register_watcher(&admin, &watcher);

    alert_client.initialize(&admin).unwrap();
    let watcher_contract_id = watcher_client.address.clone();
    alert_client
        .set_watcher_registry(&admin, &watcher_contract_id)
        .unwrap();

    alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Alert"),
        &String::from_str(&env, "hash"),
        &vec![&env],
    );

    // Watcher can read before removal
    assert_eq!(
        alert_client
            .get_alerts_for_contract(&watcher, &target)
            .unwrap()
            .len(),
        1
    );

    // Remove the watcher
    watcher_client.remove_watcher(&admin, &watcher);

    // Now rejected
    assert_eq!(
        alert_client
            .try_get_alerts_for_contract(&watcher, &target)
            .unwrap_err()
            .unwrap(),
        AlertError::NotAWatcher
    );
}

/// Watcher-gating also applies to get_alerts_by_owner.
#[test]
fn test_watcher_gating_get_alerts_by_owner() {
    let (env, alert_client, watcher_client) = setup();

    let admin = Address::generate(&env);
    let watcher = Address::generate(&env);
    let stranger = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    watcher_client.initialize(&admin);
    watcher_client.register_watcher(&admin, &watcher);

    alert_client.initialize(&admin).unwrap();
    let watcher_contract_id = watcher_client.address.clone();
    alert_client
        .set_watcher_registry(&admin, &watcher_contract_id)
        .unwrap();

    alert_client.register_alert(
        &owner,
        &target,
        &String::from_str(&env, "Alert"),
        &String::from_str(&env, "hash"),
        &vec![&env],
    );

    // Registered watcher can query by owner
    assert_eq!(
        alert_client
            .get_alerts_by_owner(&watcher, &owner)
            .unwrap()
            .len(),
        1
    );

    // Stranger is rejected
    assert_eq!(
        alert_client
            .try_get_alerts_by_owner(&stranger, &owner)
            .unwrap_err()
            .unwrap(),
        AlertError::NotAWatcher
    );
}
