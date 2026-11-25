//! Shared guard helpers for the watcher registry contract.
//!
//! Centralises the authorization and circuit-breaker checks used by
//! [`crate::WatcherRegistry`] so they are defined once and mirror the
//! `guards` module in `alert-registry`.

use soroban_sdk::{Address, Env};

use crate::{ContractError, DataKey};

/// Return `Ok(())` if `caller` is in the admin set.
///
/// Returns [`ContractError::NotInitialized`] when no admin set exists yet and
/// [`ContractError::Unauthorized`] when `caller` is not an admin.
pub fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    if !env.storage().instance().has(&DataKey::Admins) {
        return Err(ContractError::NotInitialized);
    }
    let admins: soroban_sdk::Vec<Address> = env
        .storage()
        .instance()
        .get(&DataKey::Admins)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    for i in 0..admins.len() {
        if admins.get(i).unwrap() == *caller {
            return Ok(());
        }
    }
    Err(ContractError::Unauthorized)
}

/// Return [`ContractError::ContractPaused`] when the circuit breaker is engaged.
///
/// Invoked at the top of every mutating entry point so that all writes halt
/// while read-only functions remain available.
pub fn assert_not_paused(env: &Env) -> Result<(), ContractError> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        return Err(ContractError::ContractPaused);
    }
    Ok(())
}
