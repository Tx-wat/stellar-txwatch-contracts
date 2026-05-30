use alert_registry::{AlertRegistry, AlertRegistryClient};
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

/// An authorized watcher can query AlertRegistry and see registered alerts.
#[test]
fn test_authorized_watcher_can_query_alert_registry() {
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

    // Verify the watcher is authorized
    assert!(watcher_client.is_authorized(&watcher));

    // Authorized watcher queries the alert registry
    let alerts = alert_client.get_alerts_for_contract(&target);
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

    assert!(!watcher_client.is_authorized(&stranger));
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
    assert!(!watcher_client.is_authorized(&watcher));

    // Alert data is still intact
    assert_eq!(alert_client.get_alerts_for_contract(&target).len(), 1);
}
