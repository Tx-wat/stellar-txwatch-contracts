//! Tests for the optional `note` argument of `propose_webhook` (#420).
//!
//! The note must appear in the `alert.wh_prop` event data, must not be written
//! to the stored alert, and must follow the same 128-byte limit as the label.

use crate::{AlertRegistry, AlertRegistryClient, ContractError};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    vec, Address, Env, String, Symbol, TryIntoVal, Val,
};

fn setup() -> (Env, AlertRegistryClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    let client = AlertRegistryClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let target = Address::generate(&env);
    (env, client, owner, target)
}

/// A 64-character webhook hash, the only length `register_alert` accepts.
fn hash64(env: &Env, fill: u8) -> String {
    String::from_bytes(env, &[fill; 64])
}

fn register(env: &Env, client: &AlertRegistryClient<'_>, owner: &Address, target: &Address) -> u64 {
    client.register_alert(
        owner,
        target,
        &String::from_str(env, "alert"),
        &hash64(env, b'a'),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

/// Returns the data payload of the most recent `alert.wh_prop` event, if any.
fn last_wh_prop_data(env: &Env) -> Option<Val> {
    let wanted_topic: Val = Symbol::new(env, "wh_prop").try_into_val(env).unwrap();
    let alert_topic: Val = Symbol::new(env, "alert").try_into_val(env).unwrap();
    env.events()
        .all()
        .iter()
        .rev()
        .find(|(_, topics, _)| {
            topics.len() == 2
                && topics.get(0).unwrap().shallow_eq(&alert_topic)
                && topics.get(1).unwrap().shallow_eq(&wanted_topic)
        })
        .map(|(_, _, data)| data)
}

#[test]
fn note_is_included_in_the_wh_prop_event_data() {
    let (env, client, owner, target) = setup();
    let id = register(&env, &client, &owner, &target);
    let note = String::from_str(&env, "quarterly key rotation");

    client.propose_webhook(&owner, &id, &hash64(&env, b'b'), &Some(note.clone()));

    let data = last_wh_prop_data(&env).expect("propose_webhook must emit alert.wh_prop");
    let (event_id, event_caller, event_note): (u64, Address, Option<String>) =
        data.try_into_val(&env).unwrap();
    assert_eq!(event_id, id);
    assert_eq!(event_caller, owner);
    assert_eq!(event_note, Some(note));
}

#[test]
fn absent_note_is_emitted_as_none() {
    let (env, client, owner, target) = setup();
    let id = register(&env, &client, &owner, &target);

    client.propose_webhook(&owner, &id, &hash64(&env, b'b'), &None);

    let data = last_wh_prop_data(&env).expect("propose_webhook must emit alert.wh_prop");
    let (_, _, event_note): (u64, Address, Option<String>) = data.try_into_val(&env).unwrap();
    assert_eq!(event_note, None);
}

#[test]
fn note_is_not_stored_on_the_alert() {
    let (env, client, owner, target) = setup();
    let id = register(&env, &client, &owner, &target);
    let note = String::from_str(&env, "reason-that-must-stay-off-chain");
    let new_hash = hash64(&env, b'b');

    client.propose_webhook(&owner, &id, &new_hash, &Some(note.clone()));

    let stored = client.get_alert(&id).unwrap();
    assert_eq!(stored.pending_webhook_hash, Some(new_hash));
    assert_ne!(stored.label, note);
    assert_ne!(stored.webhook_hash, note);
}

#[test]
fn note_of_exactly_128_bytes_is_accepted() {
    let (env, client, owner, target) = setup();
    let id = register(&env, &client, &owner, &target);
    let note = String::from_bytes(&env, &[b'n'; 128]);

    client.propose_webhook(&owner, &id, &hash64(&env, b'b'), &Some(note.clone()));

    let data = last_wh_prop_data(&env).expect("propose_webhook must emit alert.wh_prop");
    let (_, _, event_note): (u64, Address, Option<String>) = data.try_into_val(&env).unwrap();
    assert_eq!(event_note, Some(note));
}

#[test]
fn note_longer_than_128_bytes_is_rejected_without_state_change() {
    let (env, client, owner, target) = setup();
    let id = register(&env, &client, &owner, &target);
    let too_long = String::from_bytes(&env, &[b'n'; 129]);

    assert_eq!(
        client.try_propose_webhook(&owner, &id, &hash64(&env, b'b'), &Some(too_long),),
        Err(Ok(ContractError::LabelTooLong))
    );
    assert_eq!(client.get_alert(&id).unwrap().pending_webhook_hash, None);
}
