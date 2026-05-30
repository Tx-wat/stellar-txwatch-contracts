#![no_std]
#![warn(clippy::pedantic)]

mod contract;
mod storage;
mod types;

pub use contract::AlertRegistry;
pub use types::{AlertConfig, ContractError, DataKey};

// The Soroban SDK generates `AlertRegistryClient` in the same module as
// `#[contractimpl]`.  Re-export it so tests and integration crates can use it.
#[cfg(any(test, feature = "testutils"))]
pub use contract::AlertRegistryClient;

#[cfg(test)]
mod tests;
