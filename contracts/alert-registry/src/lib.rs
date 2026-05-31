#![no_std]
#![warn(clippy::pedantic)]

pub mod contract;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

pub use contract::AlertRegistry;
pub use types::{AlertConfig, ContractError, DataKey};
