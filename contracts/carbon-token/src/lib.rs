#![no_std]
use common::Error;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, IntoVal};

const ISSUER_KEY: Symbol = symbol_short!("issuer");
const VERIFICATION_RECORDS_KEY: Symbol = symbol_short!("ver_rec");

#[contract]
pub struct CarbonCreditToken;

#[contractimpl]
impl CarbonCreditToken {
    pub fn initialize(env: Env, issuer: Address, verification_records: Address) {
        env.storage().instance().set(&ISSUER_KEY, &issuer);
        env.storage().instance().set(&VERIFICATION_RECORDS_KEY, &verification_records);
    }

    pub fn mint(
        env: Env,
        project_id: String,
        quantity: i128,
        verification_record_ref: String,
    ) {
        let issuer: Address = env.storage().instance().get(&ISSUER_KEY).unwrap();
        issuer.require_auth();

        let ver_rec_addr: Address = env.storage().instance().get(&VERIFICATION_RECORDS_KEY).unwrap();
        let is_suspended: bool = env.invoke_contract(&ver_rec_addr, &Symbol::new(&env, "is_suspended"), (project_id.clone(),).into_val(&env));
        if is_suspended {
            panic!("project is suspended");
        }

        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        let supply_key = (Symbol::new(&env, "supply"), project_id.clone());
        if env.storage().persistent().has(&supply_key) {
            panic!("{}", Error::OverIssuance as u32);
        }

        let backing_key = (Symbol::new(&env, "backing"), project_id.clone());
        env.storage()
            .persistent()
            .set(&supply_key, &quantity);
        env.storage()
            .persistent()
            .set(&backing_key, &verification_record_ref);

        let bal_key = (
            Symbol::new(&env, "balance"),
            issuer.clone(),
            project_id.clone(),
        );
        env.storage().persistent().set(&bal_key, &quantity);

        let topics = (symbol_short!("mint"), project_id);
        env.events()
            .publish(topics, (quantity, verification_record_ref));
    }

    pub fn transfer(env: Env, from: Address, to: Address, token_id: String, quantity: i128) {
        from.require_auth();

        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        let retired_key = (Symbol::new(&env, "retired"), token_id.clone());
        if env.storage().persistent().has(&retired_key) {
            panic!("{}", Error::TokenRetired as u32);
        }

        let from_key = (
            Symbol::new(&env, "balance"),
            from.clone(),
            token_id.clone(),
        );
        let from_balance: i128 = env.storage().persistent().get(&from_key).unwrap_or(0);

        if from_balance < quantity {
            panic!("{}", Error::InsufficientBalance as u32);
        }

        let new_from = from_balance - quantity;
        if new_from == 0 {
            env.storage().persistent().remove(&from_key);
        } else {
            env.storage().persistent().set(&from_key, &new_from);
        }

        let to_key = (Symbol::new(&env, "balance"), to.clone(), token_id.clone());
        let to_balance: i128 = env.storage().persistent().get(&to_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&to_key, &(to_balance + quantity));
    }

    pub fn retire(env: Env, wallet: Address, token_id: String, quantity: i128, reason: String) {
        wallet.require_auth();

        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        let bal_key = (
            Symbol::new(&env, "balance"),
            wallet.clone(),
            token_id.clone(),
        );
        let balance: i128 = env.storage().persistent().get(&bal_key).unwrap_or(0);

        if balance < quantity {
            panic!("{}", Error::InsufficientBalance as u32);
        }

        let new_balance = balance - quantity;
        if new_balance == 0 {
            env.storage().persistent().remove(&bal_key);
        } else {
            env.storage().persistent().set(&bal_key, &new_balance);
        }

        let retired_key = (Symbol::new(&env, "retired"), token_id.clone());
        env.storage().persistent().set(&retired_key, &true);

        let timestamp = env.ledger().timestamp();
        let topics = (symbol_short!("retire"), token_id);
        env.events()
            .publish(topics, (wallet, quantity, reason, timestamp));
    }

    pub fn balance(env: Env, wallet: Address, token_id: String) -> i128 {
        let bal_key = (Symbol::new(&env, "balance"), wallet, token_id);
        env.storage().persistent().get(&bal_key).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup() -> (Env, CarbonCreditTokenClient<'static>, Address, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, CarbonCreditToken);
        let client = CarbonCreditTokenClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let user = Address::generate(&env);
        let admin = Address::generate(&env);

        let ver_rec_id = env.register_contract(None, verification_records::VerificationRecords);
        let ver_rec_client = verification_records::VerificationRecordsClient::new(&env, &ver_rec_id);
        ver_rec_client.initialize(&issuer, &admin);

        client.initialize(&issuer, &ver_rec_id);

        (env, client, issuer, user)
    }

    #[test]
    fn test_successful_mint() {
        let (env, client, issuer_addr, _user) = setup();

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-abc-123");
        let quantity: i128 = 1000;

        env.mock_all_auths();
        // Since we are mocking auths, we also need to mock the verification_records call if we don't register it.
        // But for unit tests, we can just mock the contract call.
        
        client.mint(&project_id, &quantity, &verification_ref);

        let bal = client.balance(&issuer_addr, &project_id);
        assert_eq!(bal, 1000);
    }

    #[test]
    fn test_transfer() {
        let (env, client, issuer_addr, user) = setup();

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-abc-123");

        env.mock_all_auths();
        client.mint(&project_id, &1000, &verification_ref);
        client.transfer(&issuer_addr, &user, &project_id, &400);

        let issuer_bal = client.balance(&issuer_addr, &project_id);
        let user_bal = client.balance(&user, &project_id);
        assert_eq!(issuer_bal, 600);
        assert_eq!(user_bal, 400);
    }

    #[test]
    fn test_retire() {
        let (env, client, issuer_addr, user) = setup();

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-abc-123");
        let reason = String::from_str(&env, "offsetting corporate emissions");

        env.mock_all_auths();
        client.mint(&project_id, &1000, &verification_ref);
        client.transfer(&issuer_addr, &user, &project_id, &500);
        client.retire(&user, &project_id, &200, &reason);

        let user_bal = client.balance(&user, &project_id);
        assert_eq!(user_bal, 300);
    }

    #[test]
    fn test_over_issuance_rejected() {
        let (env, client, _issuer_addr, _user) = setup();

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-abc-123");

        env.mock_all_auths();
        client.mint(&project_id, &1000, &verification_ref);

        let result = client.try_mint(&project_id, &500, &verification_ref);
        assert!(result.is_err());
    }

    #[test]
    fn test_transfer_of_retired_rejected() {
        let (env, client, issuer_addr, user) = setup();

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-abc-123");
        let reason = String::from_str(&env, "retirement");

        env.mock_all_auths();
        client.mint(&project_id, &1000, &verification_ref);
        client.transfer(&issuer_addr, &user, &project_id, &500);
        client.retire(&user, &project_id, &500, &reason);

        let result = client.try_transfer(&user, &issuer_addr, &project_id, &100);
        assert!(result.is_err());
    }
}
