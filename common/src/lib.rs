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
    /// Quantity must be positive.
    InvalidQuantity = 6,
    /// The contract has already been initialized.
    AlreadyInitialized = 7,
    /// The caller is not authorized to perform this operation.
    Unauthorized = 8,
    /// The project is suspended and cannot be minted.
    ProjectSuspended = 9,
}
