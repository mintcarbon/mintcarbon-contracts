#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, IntoVal, String, Symbol,
};

#[contracttype]
pub struct LockedEntry {
    pub seller: Address,
    pub token_id: String,
    pub quantity: i128,
}

const ADMIN: Symbol = symbol_short!("admin");
const TOKEN: Symbol = symbol_short!("token");

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    pub fn initialize(env: Env, admin: Address, token_address: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&TOKEN, &token_address);
    }

    pub fn lock(env: Env, listing_id: u32, seller: Address, token_id: String, quantity: i128) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let _: () = env.invoke_contract(
            &token_address,
            &symbol_short!("transfer"),
            (
                seller.clone(),
                env.current_contract_address(),
                token_id.clone(),
                quantity,
            )
                .into_val(&env),
        );

        let entry = LockedEntry {
            seller: seller.clone(),
            token_id: token_id.clone(),
            quantity,
        };
        env.storage().persistent().set(&listing_id, &entry);

        env.events().publish(
            (Symbol::new(&env, "escrow_locked"),),
            (listing_id, seller, token_id, quantity),
        );
    }

    pub fn release(env: Env, listing_id: u32) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let entry: LockedEntry = env.storage().persistent().get(&listing_id).unwrap();

        if entry.quantity > 0 {
            let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
            let _: () = env.invoke_contract(
                &token_address,
                &symbol_short!("transfer"),
                (
                    env.current_contract_address(),
                    entry.seller.clone(),
                    entry.token_id.clone(),
                    entry.quantity,
                )
                    .into_val(&env),
            );
        }

        env.storage().persistent().remove(&listing_id);

        env.events().publish(
            (Symbol::new(&env, "escrow_released"),),
            (listing_id, entry.seller, entry.token_id, entry.quantity),
        );
    }

    pub fn settle(env: Env, listing_id: u32, buyer: Address, quantity: i128) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        let mut entry: LockedEntry = env.storage().persistent().get(&listing_id).unwrap();

        if quantity > entry.quantity {
            panic!("insufficient locked quantity");
        }

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let _: () = env.invoke_contract(
            &token_address,
            &symbol_short!("transfer"),
            (
                env.current_contract_address(),
                buyer.clone(),
                entry.token_id.clone(),
                quantity,
            )
                .into_val(&env),
        );

        entry.quantity -= quantity;
        if entry.quantity == 0 {
            env.storage().persistent().remove(&listing_id);
        } else {
            env.storage().persistent().set(&listing_id, &entry);
        }

        env.events().publish(
            (Symbol::new(&env, "escrow_settled"),),
            (listing_id, buyer, entry.token_id, quantity),
        );
    }

    pub fn get_locked(env: Env, listing_id: u32) -> Option<LockedEntry> {
        env.storage().persistent().get(&listing_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (
        Env,
        EscrowClient<'static>,
        Address,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        let escrow_id = env.register_contract(None, Escrow);
        let escrow_client = EscrowClient::new(&env, &escrow_id);

        let ver_rec_id = env.register_contract(None, verification_records::VerificationRecords);
        let ver_rec_client =
            verification_records::VerificationRecordsClient::new(&env, &ver_rec_id);

        let token_id = env.register_contract(None, carbon_token::CarbonCreditToken);
        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);

        let marketplace = Address::generate(&env);
        let seller = Address::generate(&env);
        let buyer = Address::generate(&env);

        let verification_records_admin = Address::generate(&env);
        ver_rec_client.initialize(&marketplace, &verification_records_admin);

        token_client.initialize(&marketplace, &ver_rec_id);

        env.mock_all_auths_allowing_non_root_auth();

        escrow_client.initialize(&marketplace, &token_id);

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-001");
        token_client.mint(&project_id, &1000, &verification_ref);
        token_client.transfer(&marketplace, &seller, &project_id, &800);

        (env, escrow_client, token_id, marketplace, seller, buyer)
    }

    #[test]
    fn test_lock_and_get() {
        let (env, escrow, token_id, _marketplace, seller, _buyer) = setup();

        let project_id = String::from_str(&env, "project-001");

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);
        escrow.lock(&1, &seller, &project_id, &400);

        let locked = escrow.get_locked(&1).unwrap();
        assert_eq!(locked.seller, seller);
        assert_eq!(locked.token_id, project_id);
        assert_eq!(locked.quantity, 400);

        let seller_bal = token_client.balance(&seller, &project_id);
        assert_eq!(seller_bal, 400);
    }

    #[test]
    fn test_partial_settle() {
        let (env, escrow, token_id, _marketplace, seller, buyer) = setup();

        let project_id = String::from_str(&env, "project-001");

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);
        escrow.lock(&1, &seller, &project_id, &400);

        escrow.settle(&1, &buyer, &150);
        let buyer_bal = token_client.balance(&buyer, &project_id);
        assert_eq!(buyer_bal, 150);

        let locked = escrow.get_locked(&1).unwrap();
        assert_eq!(locked.quantity, 250);

        escrow.settle(&1, &buyer, &250);
        let buyer_bal = token_client.balance(&buyer, &project_id);
        assert_eq!(buyer_bal, 400);

        assert!(escrow.get_locked(&1).is_none());
    }

    #[test]
    fn test_release() {
        let (env, escrow, token_id, _marketplace, seller, _buyer) = setup();

        let project_id = String::from_str(&env, "project-001");

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);
        escrow.lock(&1, &seller, &project_id, &400);
        escrow.release(&1);

        let seller_bal = token_client.balance(&seller, &project_id);
        assert_eq!(seller_bal, 800);

        assert!(escrow.get_locked(&1).is_none());
    }

    #[test]
    fn test_release_after_partial_settle() {
        let (env, escrow, token_id, _marketplace, seller, buyer) = setup();

        let project_id = String::from_str(&env, "project-001");

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);
        escrow.lock(&1, &seller, &project_id, &400);
        escrow.settle(&1, &buyer, &150);

        escrow.release(&1);

        let seller_bal = token_client.balance(&seller, &project_id);
        assert_eq!(seller_bal, 650);

        assert!(escrow.get_locked(&1).is_none());
    }

    #[test]
    fn test_settle() {
        let (env, escrow, token_id, _marketplace, seller, buyer) = setup();

        let project_id = String::from_str(&env, "project-001");

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);
        escrow.lock(&1, &seller, &project_id, &400);
        escrow.settle(&1, &buyer, &400);

        let buyer_bal = token_client.balance(&buyer, &project_id);
        assert_eq!(buyer_bal, 400);

        let seller_bal = token_client.balance(&seller, &project_id);
        assert_eq!(seller_bal, 400);

        assert!(escrow.get_locked(&1).is_none());
    }

    #[test]
    fn test_lock_insufficient_balance() {
        let (env, escrow, _token_id, _marketplace, seller, _buyer) = setup();

        let project_id = String::from_str(&env, "project-002");

        let result = escrow.try_lock(&1, &seller, &project_id, &100);
        assert!(result.is_err());
    }
}
