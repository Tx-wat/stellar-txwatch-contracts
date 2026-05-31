#[cfg(test)]
mod tests {
    use crate::contract::AlertRegistry;
    use crate::types::ContractError;
    use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

    // Re-export the generated client so tests can use it.
    use soroban_sdk::contract;
    #[allow(unused_imports)]
    use crate::contract::AlertRegistryClient;

    fn setup() -> (Env, AlertRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AlertRegistry);
        let client = AlertRegistryClient::new(&env, &contract_id);
        (env, client)
    }

    fn str(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

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
        client.initialize(&admin).unwrap();

        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:mint")],
        );

        assert_eq!(client.remove_alert_by_admin(&admin, &id), Ok(()));
        assert!(client.get_alert(&id).is_none());
    }

    #[test]
    #[should_panic(expected = "owner alert limit exceeded")]
    fn test_admin_set_per_owner_alert_limit() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin).unwrap();
        client.set_per_owner_alert_limit(&admin, &1u32).unwrap();

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
        client.initialize(&admin).unwrap();
        let new_admin = Address::generate(&env);

        assert_eq!(client.transfer_admin(&admin, &new_admin), Ok(()));
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        assert_eq!(client.remove_alert_by_admin(&new_admin, &id), Ok(()));
    }

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

    #[test]
    fn test_get_nonexistent_alert() {
        let (_env, client) = setup();
        assert!(client.get_alert(&999u64).is_none());
    }

    #[test]
    fn test_get_alerts_for_contract_empty() {
        let (env, client) = setup();
        let target = Address::generate(&env);
        let result = client.get_alerts_for_contract(&target);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_index_queries() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        client.register_alert(&owner, &target, &str(&env, "A1"), &str(&env, "h1"), &vec![&env]);
        client.register_alert(&owner, &target, &str(&env, "A2"), &str(&env, "h2"), &vec![&env]);

        assert_eq!(client.get_alerts_for_contract(&target).len(), 2);
        assert_eq!(client.get_alerts_by_owner(&owner).len(), 2);
    }

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

    #[test]
    fn test_update_webhook() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "old-hash"),
            &vec![&env],
        );
        assert_eq!(
            client
                .try_update_webhook(&owner, &id, &str(&env, "new-hash"))
                .unwrap(),
            Ok(())
        );
        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.webhook_hash, str(&env, "new-hash"));
    }

    #[test]
    fn test_update_webhook_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "hash"),
            &vec![&env],
        );
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

    #[test]
    fn test_active_defaults_to_true() {
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

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.active, true);
    }

    #[test]
    #[should_panic(expected = "too many rules: maximum is 50")]
    fn test_register_alert_too_many_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let mut rules: soroban_sdk::Vec<String> = vec![&env];
        for _ in 0..51u32 {
            rules.push_back(str(&env, "rule:transfer"));
        }
        client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
    }

    #[test]
    #[should_panic(expected = "too many rules: maximum is 50")]
    fn test_update_alert_too_many_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "h"),
            &vec![&env],
        );

        let mut rules: soroban_sdk::Vec<String> = vec![&env];
        for _ in 0..51u32 {
            rules.push_back(str(&env, "rule:transfer"));
        }
        client.update_alert(&owner, &id, &rules, &true);
    }

    #[test]
    fn test_register_alert_exactly_50_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let mut rules: soroban_sdk::Vec<String> = vec![&env];
        for _ in 0..50u32 {
            rules.push_back(str(&env, "rule:transfer"));
        }
        let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.rules.len(), 50);
    }

    #[test]
    #[should_panic(expected = "label exceeds 128 bytes")]
    fn test_label_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let long_label = str(&env, &"a".repeat(129));
        client.register_alert(&owner, &target, &long_label, &str(&env, "hash"), &vec![&env]);
    }

    #[test]
    fn test_label_max_length_accepted() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let max_label = str(&env, &"a".repeat(128));
        client.register_alert(&owner, &target, &max_label, &str(&env, "hash"), &vec![&env]);
    }
}
