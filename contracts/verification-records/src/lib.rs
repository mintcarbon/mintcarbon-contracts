#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub registry: String,
    pub cert_id: String,
    pub project_id: String,
    pub timestamp: u64,
    pub suspended: bool,
}

#[contract]
pub struct VerificationRecords;

const ISSUER_KEY: Symbol = symbol_short!("issuer");
const ADMIN_KEY: Symbol = symbol_short!("admin");

#[contractimpl]
impl VerificationRecords {
    pub fn initialize(env: Env, issuer: Address, admin: Address) {
        if env.storage().instance().has(&ISSUER_KEY) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ISSUER_KEY, &issuer);
        env.storage().instance().set(&ADMIN_KEY, &admin);
    }

    pub fn create_record(env: Env, registry: String, cert_id: String, project_id: String) {
        let issuer: Address = env
            .storage()
            .instance()
            .get(&ISSUER_KEY)
            .expect("not initialized");
        issuer.require_auth();

        let key = (symbol_short!("record"), project_id.clone());
        if env.storage().persistent().has(&key) {
            panic!("record already exists");
        }

        let record = Record {
            registry,
            cert_id,
            project_id: project_id.clone(),
            timestamp: env.ledger().timestamp(),
            suspended: false,
        };

        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (symbol_short!("rec_creat"), project_id),
            (record.registry, record.cert_id, record.timestamp),
        );
    }

    pub fn suspend(env: Env, project_id: String) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("not initialized");
        admin.require_auth();

        let key = (symbol_short!("record"), project_id.clone());
        let mut record: Record = env
            .storage()
            .persistent()
            .get(&key)
            .expect("record not found");

        record.suspended = true;
        env.storage().persistent().set(&key, &record);

        env.events()
            .publish((symbol_short!("cert_rev"), project_id), ());
    }

    pub fn get_record(env: Env, project_id: String) -> Record {
        let key = (symbol_short!("record"), project_id);
        env.storage()
            .persistent()
            .get(&key)
            .expect("record not found")
    }

    pub fn is_suspended(env: Env, project_id: String) -> bool {
        let key = (symbol_short!("record"), project_id);
        if let Some(record) = env.storage().persistent().get::<_, Record>(&key) {
            record.suspended
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup() -> (Env, VerificationRecordsClient<'static>, Address, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerificationRecords);
        let client = VerificationRecordsClient::new(&env, &contract_id);

        let issuer = Address::generate(&env);
        let admin = Address::generate(&env);

        client.initialize(&issuer, &admin);

        (env, client, issuer, admin)
    }

    #[test]
    fn test_create_record() {
        let (env, client, _issuer, _admin) = setup();

        let registry = String::from_str(&env, "Verra");
        let cert_id = String::from_str(&env, "V-123");
        let project_id = String::from_str(&env, "P-001");

        env.mock_all_auths();
        client.create_record(&registry, &cert_id, &project_id);

        let record = client.get_record(&project_id);
        assert_eq!(record.registry, registry);
        assert_eq!(record.cert_id, cert_id);
        assert_eq!(record.project_id, project_id);
        assert!(!record.suspended);
    }

    #[test]
    fn test_suspend() {
        let (env, client, _issuer, _admin) = setup();

        let registry = String::from_str(&env, "Verra");
        let cert_id = String::from_str(&env, "V-123");
        let project_id = String::from_str(&env, "P-001");

        env.mock_all_auths();
        client.create_record(&registry, &cert_id, &project_id);
        assert!(!client.is_suspended(&project_id));

        client.suspend(&project_id);
        assert!(client.is_suspended(&project_id));
    }

    #[test]
    #[should_panic(expected = "record already exists")]
    fn test_duplicate_record() {
        let (env, client, _issuer, _admin) = setup();

        let registry = String::from_str(&env, "Verra");
        let cert_id = String::from_str(&env, "V-123");
        let project_id = String::from_str(&env, "P-001");

        env.mock_all_auths();
        client.create_record(&registry, &cert_id, &project_id);
        client.create_record(&registry, &cert_id, &project_id);
    }
}
