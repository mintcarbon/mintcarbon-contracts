use audit_log::AuditLogClient;
use carbon_token::CarbonCreditTokenClient;
use escrow::EscrowClient;
use governance::GovernanceClient;
use marketplace::MarketplaceClient;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use verification_records::VerificationRecordsClient;

#[test]
fn test_full_flow() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    // 1. Register all contracts
    let carbon_token_id = env.register_contract(None, carbon_token::CarbonCreditToken);
    let marketplace_id = env.register_contract(None, marketplace::Marketplace);
    let escrow_id = env.register_contract(None, escrow::Escrow);
    let verification_records_id =
        env.register_contract(None, verification_records::VerificationRecords);
    let audit_log_id = env.register_contract(None, audit_log::AuditLog);
    let governance_id = env.register_contract(None, governance::Governance);

    // Clients
    let carbon_token = CarbonCreditTokenClient::new(&env, &carbon_token_id);
    let marketplace = MarketplaceClient::new(&env, &marketplace_id);
    let escrow = EscrowClient::new(&env, &escrow_id);
    let verification_records = VerificationRecordsClient::new(&env, &verification_records_id);
    let audit_log = AuditLogClient::new(&env, &audit_log_id);
    let governance = GovernanceClient::new(&env, &governance_id);

    // Addresses
    let issuer = Address::generate(&env);
    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);

    // 2. Initialize contracts
    carbon_token.initialize(&issuer, &verification_records_id);
    verification_records.initialize(&issuer, &admin);
    audit_log.initialize(&admin);
    escrow.initialize(&marketplace_id, &carbon_token_id);

    let native_asset = env.register_stellar_asset_contract_v2(issuer.clone());
    marketplace.initialize(&escrow_id, &carbon_token_id, &native_asset.address());

    let mut admins = Vec::new(&env);
    admins.push_back(admin.clone());
    admins.push_back(Address::generate(&env));
    admins.push_back(Address::generate(&env));
    governance.initialize(&admins, &172800, &audit_log_id);

    // 3. Full Flow
    let project_id = String::from_str(&env, "project-X");
    let registry = String::from_str(&env, "Verra");
    let cert_id = String::from_str(&env, "V-123");

    // a. Create verification record
    verification_records.create_record(&registry, &cert_id, &project_id);

    // b. Mint tokens
    let quantity: i128 = 1000;
    carbon_token.mint(&project_id, &quantity, &cert_id);

    // c. Transfer to seller
    carbon_token.transfer(&issuer, &seller, &project_id, &500);

    // d. Seller creates listing
    let price: i128 = 10_000_000; // 10 XLM per token
    let listing_id = marketplace.create_listing(&seller, &project_id, &100, &price);

    // e. Buyer gets XLM
    let sac = StellarAssetClient::new(&env, &native_asset.address());
    sac.mint(&buyer, &2_000_000_000);

    // f. Buyer places order
    marketplace.place_order(&buyer, &listing_id, &100);

    // g. Buyer retires tokens
    let reason = String::from_str(&env, "Offsetting flights");
    carbon_token.retire(&buyer, &project_id, &50, &reason);

    // Assertions
    assert_eq!(carbon_token.balance(&buyer, &project_id), 50);
    assert_eq!(carbon_token.balance(&seller, &project_id), 400);

    // Check AuditLog via Governance object
    governance.object(&buyer, &String::from_str(&env, "Suspicious activity"));
    assert!(audit_log.get_count() >= 1);
    assert!(audit_log.verify_chain(&0, &(audit_log.get_count() - 1)));

    // 4. Test revocation/suspension
    verification_records.suspend(&project_id);
    assert!(verification_records.is_suspended(&project_id));

    // Attempt mint after suspension should fail
    let result = carbon_token.try_mint(&project_id, &100, &cert_id);
    assert!(result.is_err());
}
