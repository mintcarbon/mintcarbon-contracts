#![no_std]
use soroban_sdk::{contracterror, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotAuthorized = 1,
    InsufficientBalance = 2,
    TokenRetired = 3,
    OverIssuance = 4,
    NotFound = 5,
}

pub fn require_auth(_env: &Env, address: &Address) {
    address.require_auth();
}
