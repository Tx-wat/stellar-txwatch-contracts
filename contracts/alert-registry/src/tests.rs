use crate::AlertRegistry;
use crate::ContractError;
use crate::AlertRegistryClient;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

/// Build a Soroban String of `n` repetitions of ASCII char `ch`.
/// Uses a fixed 8192-byte stack buffer — sufficient for the Soroban max.
fn str_repeat(env: &Env, ch: char, n: usize) -> String {
    assert!(n <= 8192, "str_repeat: n exceeds Soroban String max");
    let byte = ch as u8;
    let mut buf = [0u8; 8192];
    for b in buf.iter_mut().take(n) {
        *b = byte;
    }
    let s = core::str::from_utf8(&buf[..n]).unwrap();
    String::from_str(env, s)
}

// 1. Happy path — register and retrieve
#[test]
fn test_register_and_get_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "My Alert"),
        &str(&env, "hash123"),
        &vec![&env, str(&env, "rule:transfer")],
    );

    let cfg = client.get_alert(&id).unwrap();
    assert_eq!(cfg.label, str(&env, "My Alert"));
    assert_eq!(cfg.owner, owner);
    assert!(cfg.active);
}

// 2. Happy path — update alert
#[test]
fn test_update_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env, str(&env, "rule:transfer")],
    );

    assert_eq!(
        client
            .try_update_alert(&owner, &id, &vec![&env, str(&env, "rule:mint")], &false)
            .unwrap(),
        Ok(())
    );

    let cfg = client.get_alert(&id).unwrap();
    assert!(!cfg.active);
    assert_eq!(cfg.rules.get(0).unwrap(), str(&env, "rule:mint"));
}

// 3. Happy path — remove alert
#[test]
fn test_remove_alert() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env],
    );

    assert_eq!(client.try_remove_alert(&owner, &id).unwrap(), Ok(()));
    assert!(client.get_alert(&id).is_none());
}

// 4. Unauthorized update rejected
#[test]
fn test_update_unauthorized() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env],
    );

    assert_eq!(
        client
            .try_update_alert(&attacker, &id, &vec![&env], &false)
            .unwrap_err()
            .unwrap(),
        ContractError::Unauthorized
    );
}

#[test]
#[should_panic(expected = "invalid rule descriptor")]
fn test_register_alert_rejects_invalid_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env, str(&env, "rule:unknown")],
    );
}

#[test]
#[should_panic(expected = "invalid rule descriptor")]
fn test_update_alert_rejects_invalid_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env, str(&env, "rule:transfer")],
    );

    client.update_alert(&owner, &id, &vec![&env, str(&env, "rule:bogus")], &true);
}

#[test]
fn test_admin_remove_any_alert() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env, str(&env, "rule:mint")],
    );

    assert_eq!(client.remove_alert_by_admin(&admin, &id), ());
    assert!(client.get_alert(&id).is_none());
}

#[test]
#[should_panic(expected = "owner alert limit exceeded")]
fn test_admin_set_per_owner_alert_limit() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.set_per_owner_alert_limit(&admin, &1u32);

    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert1"),
        &str(&env, "hash1"),
        &vec![&env, str(&env, "rule:transfer")],
    );
    client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert2"),
        &str(&env, "hash2"),
        &vec![&env, str(&env, "rule:mint")],
    );
}

#[test]
fn test_admin_transfer_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let new_admin = Address::generate(&env);

    assert_eq!(client.transfer_admin(&admin, &new_admin), ());
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env, str(&env, "rule:transfer")],
    );
    assert_eq!(client.remove_alert_by_admin(&new_admin, &id), ());
}

// 5. Unauthorized remove rejected
#[test]
fn test_remove_unauthorized() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(
        &owner,
        &target,
        &str(&env, "Alert"),
        &str(&env, "hash"),
        &vec![&env],
    );

    assert_eq!(
        client
            .try_remove_alert(&attacker, &id)
            .unwrap_err()
            .unwrap(),
        ContractError::Unauthorized
    );
}

// 6. Edge case — get nonexistent alert returns None
#[test]
fn test_get_nonexistent_alert() {
    let (_env, client) = setup();
    assert!(client.get_alert(&999u64).is_none());
}

// 7. Edge case — get alerts for contract with no alerts returns empty vec
#[test]
fn test_get_alerts_for_contract_empty() {
    let (env, client) = setup();
    let querier = Address::generate(&env);
    let target = Address::generate(&env);
    assert_eq!(client.get_alerts_for_contract(&querier, &target).unwrap().len(), 0);
}

// 8. Index queries
#[test]
fn test_index_queries() {
    let (env, client) = setup();
    let querier = Address::generate(&env);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    client.register_alert(&owner, &target, &str(&env, "A1"), &str(&env, "h1"), &vec![&env]);
    client.register_alert(&owner, &target, &str(&env, "A2"), &str(&env, "h2"), &vec![&env]);

    assert_eq!(client.get_alerts_for_contract(&querier, &target).unwrap().len(), 2);
    assert_eq!(client.get_alerts_by_owner(&querier, &owner).unwrap().len(), 2);
}

// 9. get_alert_count is monotonic
#[test]
fn test_get_alert_count() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    assert_eq!(client.get_alert_count(), 0);
    let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
    assert_eq!(client.get_alert_count(), 1);
    client.register_alert(&owner, &target, &str(&env, "B"), &str(&env, "h"), &vec![&env]);
    assert_eq!(client.get_alert_count(), 2);
    client.remove_alert(&owner, &id);
    assert_eq!(client.get_alert_count(), 2);
}

// get_active_alert_count decreases after remove
#[test]
fn test_get_active_alert_count() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    assert_eq!(client.get_active_alert_count(&owner), 0);
    let id1 = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
    let _id2 = client.register_alert(&owner, &target, &str(&env, "B"), &str(&env, "h"), &vec![&env]);
    assert_eq!(client.get_active_alert_count(&owner), 2);
    client.remove_alert(&owner, &id1);
    assert_eq!(client.get_active_alert_count(&owner), 1);
}

// 10. update_webhook changes the hash
#[test]
fn test_update_webhook() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "old-hash"), &vec![&env]);
    assert_eq!(
        client.try_update_webhook(&owner, &id, &str(&env, "new-hash")).unwrap(),
        Ok(())
    );
    assert_eq!(client.get_alert(&id).unwrap().webhook_hash, str(&env, "new-hash"));
}

// 11. update_webhook unauthorized
#[test]
fn test_update_webhook_unauthorized() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "hash"), &vec![&env]);
    assert_eq!(
        client
            .try_update_webhook(&attacker, &id, &str(&env, "evil-hash"))
            .unwrap_err()
            .unwrap(),
        ContractError::Unauthorized
    );
}

#[test]
fn test_update_alert_missing_returns_not_found() {
    let (env, client) = setup();
    let attacker = Address::generate(&env);

    assert_eq!(
        client
            .try_update_alert(&attacker, &999u64, &vec![&env], &false)
            .unwrap_err()
            .unwrap(),
        ContractError::AlertNotFound
    );
}

// 12. active defaults to true on registration
#[test]
fn test_active_defaults_to_true() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(&owner, &target, &str(&env, "Alert"), &str(&env, "hash"), &vec![&env]);
    assert!(client.get_alert(&id).unwrap().active);
}

// 13. register_alert rejects more than 50 rules
#[test]
#[should_panic(expected = "too many rules: maximum is 50")]
fn test_register_alert_too_many_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let mut rules: Vec<String> = vec![&env];
    for _ in 0..51u32 {
        rules.push_back(str(&env, "rule"));
    }
    client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
}

// 14. update_alert rejects more than 50 rules
#[test]
#[should_panic(expected = "too many rules: maximum is 50")]
fn test_update_alert_too_many_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);

    let mut rules: Vec<String> = vec![&env];
    for _ in 0..51u32 {
        rules.push_back(str(&env, "rule"));
    }
    client.update_alert(&owner, &id, &rules, &true);
}

// 15. exactly 50 rules is accepted
#[test]
fn test_register_alert_exactly_50_rules() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);

    let mut rules: Vec<String> = vec![&env];
    for i in 0..50u32 {
        rules.push_back(str(&env, if i % 2 == 0 { "rule:transfer" } else { "rule:mint" }));
    }
    let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
    assert_eq!(client.get_alert(&id).unwrap().rules.len(), 50);
}

// 16. Label exceeding 128 bytes is rejected
#[test]
#[should_panic(expected = "label exceeds 128 bytes")]
fn test_label_too_long() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let long_label = str(&env, &"a".repeat(129));
    client.register_alert(&owner, &target, &long_label, &str(&env, "hash"), &vec![&env]);
}

// 17. Label at exactly 128 bytes is accepted
#[test]
fn test_label_max_length_accepted() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let max_label = str(&env, &"a".repeat(128));
    client.register_alert(&owner, &target, &max_label, &str(&env, "hash"), &vec![&env]);
}

// ── Soroban string-length boundary tests ─────────────────────────────────────
//
// Soroban's String type supports up to 8 192 bytes.  The contract enforces
// its own tighter 128-byte limit on `label`, so any string longer than 128
// bytes must be rejected by the contract guard long before the Soroban
// limit is reached.

// 18. Label of 8 192 bytes (Soroban max) is rejected by the app guard.
#[test]
#[should_panic(expected = "label exceeds 128 bytes")]
fn test_label_at_soroban_max_rejected_by_app_guard() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let label = str_repeat(&env, 'a', 8192);
    client.register_alert(&owner, &target, &label, &str(&env, "hash"), &vec![&env]);
}

// 19. Label of 8 191 bytes (one below Soroban max) is also rejected by the app guard.
#[test]
#[should_panic(expected = "label exceeds 128 bytes")]
fn test_label_one_below_soroban_max_rejected_by_app_guard() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    let label = str_repeat(&env, 'b', 8191);
    client.register_alert(&owner, &target, &label, &str(&env, "hash"), &vec![&env]);
}

// 20. A Soroban String of exactly 8 192 bytes can be constructed without panicking.
#[test]
fn test_soroban_string_8192_bytes_is_constructible() {
    let (env, _client) = setup();
    let s = str_repeat(&env, 'x', 8192);
    assert_eq!(s.len(), 8192);
}
