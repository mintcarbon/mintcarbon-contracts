#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    pub fn placeholder() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
