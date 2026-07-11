#![no_std]
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Insufficient token balance for the requested operation.
    InsufficientBalance = 2,
    /// Token has been permanently retired and cannot be transferred.
    TokenRetired = 3,
    /// Attempted to mint tokens for a project that already has a supply.
    OverIssuance = 4,
    /// Requested resource was not found in storage.
    NotFound = 5,
}
