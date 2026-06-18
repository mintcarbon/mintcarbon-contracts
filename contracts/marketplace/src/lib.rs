#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token::TokenClient, Address, Env, IntoVal,
    String, Symbol,
};

#[contracttype]
pub struct ListingEntry {
    pub seller: Address,
    pub token_id: String,
    pub quantity: i128,
    pub price: i128,
    pub filled: i128,
}

const ESROW: Symbol = symbol_short!("escrow");
const TOKEN: Symbol = symbol_short!("token");
const NATIVE: Symbol = symbol_short!("native");
const COUNTER: Symbol = symbol_short!("counter");

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    pub fn initialize(
        env: Env,
        escrow_address: Address,
        token_address: Address,
        native_asset_address: Address,
    ) {
        env.storage().instance().set(&ESROW, &escrow_address);
        env.storage().instance().set(&TOKEN, &token_address);
        env.storage().instance().set(&NATIVE, &native_asset_address);
    }

    pub fn create_listing(
        env: Env,
        seller: Address,
        token_id: String,
        quantity: i128,
        price: i128,
    ) -> u32 {
        if quantity <= 0 || price <= 0 {
            panic!("quantity and price must be positive");
        }

        seller.require_auth();

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let balance: i128 = env.invoke_contract(
            &token_address,
            &symbol_short!("balance"),
            (seller.clone(), token_id.clone()).into_val(&env),
        );
        if balance < quantity {
            panic!("insufficient balance");
        }

        let escrow_address: Address = env.storage().instance().get(&ESROW).unwrap();
        let mut counter: u32 = env.storage().instance().get(&COUNTER).unwrap_or(0);
        counter += 1;
        env.storage().instance().set(&COUNTER, &counter);

        let _: () = env.invoke_contract(
            &escrow_address,
            &symbol_short!("lock"),
            (counter, seller.clone(), token_id.clone(), quantity).into_val(&env),
        );

        let entry = ListingEntry {
            seller: seller.clone(),
            token_id: token_id.clone(),
            quantity,
            price,
            filled: 0,
        };
        env.storage().persistent().set(&counter, &entry);

        let topics = (Symbol::new(&env, "listing_created"),);
        env.events()
            .publish(topics, (counter, seller, token_id, quantity, price));

        counter
    }

    pub fn cancel_listing(env: Env, listing_id: u32) {
        let entry: ListingEntry = env.storage().persistent().get(&listing_id).unwrap();

        entry.seller.require_auth();

        let escrow_address: Address = env.storage().instance().get(&ESROW).unwrap();
        let _: () = env.invoke_contract(
            &escrow_address,
            &symbol_short!("release"),
            (listing_id,).into_val(&env),
        );

        env.storage().persistent().remove(&listing_id);

        let topics = (Symbol::new(&env, "listing_cancelled"),);
        env.events().publish(topics, (listing_id, entry.seller));
    }

    pub fn place_order(env: Env, buyer: Address, listing_id: u32, quantity: i128) {
        if quantity <= 0 {
            panic!("quantity must be positive");
        }

        buyer.require_auth();

        let mut entry: ListingEntry = env.storage().persistent().get(&listing_id).unwrap();

        let available = entry.quantity - entry.filled;
        if quantity > available {
            panic!("insufficient available quantity");
        }

        let total_price = quantity * entry.price;

        let native_address: Address = env.storage().instance().get(&NATIVE).unwrap();
        let native_client = TokenClient::new(&env, &native_address);
        native_client.transfer(&buyer, &entry.seller, &total_price);

        let escrow_address: Address = env.storage().instance().get(&ESROW).unwrap();
        let _: () = env.invoke_contract(
            &escrow_address,
            &symbol_short!("settle"),
            (listing_id, buyer.clone(), quantity).into_val(&env),
        );

        entry.filled += quantity;
        if entry.filled == entry.quantity {
            env.storage().persistent().remove(&listing_id);
        } else {
            env.storage().persistent().set(&listing_id, &entry);
        }

        let topics = (Symbol::new(&env, "order_matched"),);
        env.events().publish(
            topics,
            (listing_id, buyer, quantity, total_price, entry.seller),
        );
    }

    pub fn get_listing(env: Env, listing_id: u32) -> Option<ListingEntry> {
        env.storage().persistent().get(&listing_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::token::StellarAssetClient;

    fn setup() -> (
        Env,
        MarketplaceClient<'static>,
        Address,
        Address,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();

        let ver_rec_id = env.register_contract(None, verification_records::VerificationRecords);
        let ver_rec_client =
            verification_records::VerificationRecordsClient::new(&env, &ver_rec_id);

        let token_id = env.register_contract(None, carbon_token::CarbonCreditToken);
        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &token_id);

        let escrow_id = env.register_contract(None, escrow::Escrow);
        let escrow_client = escrow::EscrowClient::new(&env, &escrow_id);

        let marketplace_id = env.register_contract(None, Marketplace);
        let marketplace_client = MarketplaceClient::new(&env, &marketplace_id);

        let issuer = Address::generate(&env);
        let seller = Address::generate(&env);
        let buyer = Address::generate(&env);
        let ver_rec_admin = Address::generate(&env);

        ver_rec_client.initialize(&issuer, &ver_rec_admin);
        token_client.initialize(&issuer, &ver_rec_id);

        let native_asset = env.register_stellar_asset_contract_v2(issuer.clone());
        let sac = StellarAssetClient::new(&env, &native_asset.address());

        env.mock_all_auths_allowing_non_root_auth();

        escrow_client.initialize(&marketplace_id, &token_id);
        marketplace_client.initialize(&escrow_id, &token_id, &native_asset.address());

        let project_id = String::from_str(&env, "project-001");
        let verification_ref = String::from_str(&env, "verra-001");
        token_client.mint(&project_id, &1000, &verification_ref);
        token_client.transfer(&issuer, &seller, &project_id, &500);

        sac.mint(&buyer, &1_000_000_000_000);

        (
            env,
            marketplace_client,
            token_id,
            native_asset.address(),
            issuer,
            seller,
            buyer,
        )
    }

    #[test]
    fn test_create_listing() {
        let (env, marketplace, _token_id, _native, _issuer, seller, _buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        let listing_id = marketplace.create_listing(&seller, &token_id, &100, &10_000_000);

        assert_eq!(listing_id, 1);

        let listing = marketplace.get_listing(&1).unwrap();
        assert_eq!(listing.seller, seller);
        assert_eq!(listing.quantity, 100);
        assert_eq!(listing.price, 10_000_000);
        assert_eq!(listing.filled, 0);
    }

    #[test]
    fn test_create_listing_insufficient_balance() {
        let (env, marketplace, _token_id, _native, _issuer, _seller, _buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        let result = marketplace.try_create_listing(&_seller, &token_id, &999_999, &10_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_listing() {
        let (env, marketplace, _token_id, _native, _issuer, seller, _buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        marketplace.create_listing(&seller, &token_id, &100, &10_000_000);
        marketplace.cancel_listing(&1);

        assert!(marketplace.get_listing(&1).is_none());
    }

    #[test]
    fn test_place_order_full_fill() {
        let (env, marketplace, _token_id, _native, _issuer, seller, buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        marketplace.create_listing(&seller, &token_id, &100, &10_000_000);

        marketplace.place_order(&buyer, &1, &100);

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &_token_id);
        let buyer_bal = token_client.balance(&buyer, &token_id);
        assert_eq!(buyer_bal, 100);

        let seller_bal = token_client.balance(&seller, &token_id);
        assert_eq!(seller_bal, 400);

        assert!(marketplace.get_listing(&1).is_none());
    }

    #[test]
    fn test_place_order_partial_fill() {
        let (env, marketplace, _token_id, _native, _issuer, seller, buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        marketplace.create_listing(&seller, &token_id, &100, &10_000_000);

        marketplace.place_order(&buyer, &1, &40);

        let token_client = carbon_token::CarbonCreditTokenClient::new(&env, &_token_id);
        let buyer_bal = token_client.balance(&buyer, &token_id);
        assert_eq!(buyer_bal, 40);

        let listing = marketplace.get_listing(&1).unwrap();
        assert_eq!(listing.filled, 40);

        marketplace.place_order(&buyer, &1, &60);
        let buyer_bal = token_client.balance(&buyer, &token_id);
        assert_eq!(buyer_bal, 100);

        assert!(marketplace.get_listing(&1).is_none());
    }

    #[test]
    fn test_place_order_over_available() {
        let (env, marketplace, _token_id, _native, _issuer, _seller, buyer) = setup();

        let token_id = String::from_str(&env, "project-001");
        marketplace.create_listing(&_seller, &token_id, &100, &10_000_000);

        let result = marketplace.try_place_order(&buyer, &1, &200);
        assert!(result.is_err());
    }
}
