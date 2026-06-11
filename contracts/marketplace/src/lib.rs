#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    pub fn placeholder() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
