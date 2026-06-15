#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Bytes, BytesN};
use soroban_sdk::xdr::ToXdr;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub data: String,
    pub prev_hash: BytesN<32>,
    pub entry_hash: BytesN<32>,
}

#[contract]
pub struct AuditLog;

const ENTRY_COUNT: Symbol = symbol_short!("count");
const ADMIN_KEY: Symbol = symbol_short!("admin");

#[contractimpl]
impl AuditLog {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&ENTRY_COUNT, &0u32);
    }

    pub fn append(env: Env, data: String) -> u32 {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).expect("not initialized");
        admin.require_auth();

        let count: u32 = env.storage().instance().get(&ENTRY_COUNT).unwrap_or(0);
        
        let prev_hash = if count == 0 {
            BytesN::from_array(&env, &[0u8; 32])
        } else {
            let prev_entry: AuditEntry = env.storage().persistent().get(&(symbol_short!("entry"), count - 1)).unwrap();
            prev_entry.entry_hash
        };

        // entry_hash = sha256(index || data || prev_hash)
        let mut hash_data = Bytes::new(&env);
        let index_bytes: [u8; 4] = count.to_be_bytes();
        hash_data.append(&Bytes::from_array(&env, &index_bytes));
        hash_data.append(&data.clone().to_xdr(&env));
        hash_data.append(&prev_hash.clone().into());
        
        let entry_hash = env.crypto().sha256(&hash_data);

        let entry = AuditEntry {
            data,
            prev_hash,
            entry_hash: entry_hash.clone().into(),
        };

        let key = (symbol_short!("entry"), count);
        if env.storage().persistent().has(&key) {
            panic!("entry already exists");
        }

        env.storage().persistent().set(&key, &entry);
        env.storage().instance().set(&ENTRY_COUNT, &(count + 1));

        env.events().publish((symbol_short!("audit"), count), entry_hash);

        count
    }

    pub fn get_entry(env: Env, index: u32) -> AuditEntry {
        env.storage().persistent().get(&(symbol_short!("entry"), index)).expect("entry not found")
    }

    pub fn get_count(env: Env) -> u32 {
        env.storage().instance().get(&ENTRY_COUNT).unwrap_or(0)
    }

    pub fn get_root(env: Env) -> BytesN<32> {
        let count: u32 = env.storage().instance().get(&ENTRY_COUNT).unwrap_or(0);
        if count == 0 {
            BytesN::from_array(&env, &[0u8; 32])
        } else {
            let last_entry: AuditEntry = env.storage().persistent().get(&(symbol_short!("entry"), count - 1)).unwrap();
            last_entry.entry_hash
        }
    }

    pub fn verify_chain(env: Env, from: u32, to: u32) -> bool {
        let count: u32 = env.storage().instance().get(&ENTRY_COUNT).unwrap_or(0);
        if to >= count || from > to {
            return false;
        }

        for i in from..=to {
            let entry: AuditEntry = env.storage().persistent().get(&(symbol_short!("entry"), i)).expect("entry not found");
            
            let mut hash_data = Bytes::new(&env);
            let index_bytes: [u8; 4] = i.to_be_bytes();
            hash_data.append(&Bytes::from_array(&env, &index_bytes));
            hash_data.append(&entry.data.to_xdr(&env));
            hash_data.append(&entry.prev_hash.clone().into());
            
            let computed_hash: BytesN<32> = env.crypto().sha256(&hash_data).into();
            if computed_hash != entry.entry_hash {
                return false;
            }

            if i > from {
                let prev_entry: AuditEntry = env.storage().persistent().get(&(symbol_short!("entry"), i - 1)).expect("prev entry not found");
                if entry.prev_hash != prev_entry.entry_hash {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn setup() -> (Env, AuditLogClient<'static>, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuditLog);
        let client = AuditLogClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        (env, client, admin)
    }

    #[test]
    fn test_append_and_verify() {
        let (env, client, _admin) = setup();

        env.mock_all_auths();
        client.append(&String::from_str(&env, "event 1"));
        client.append(&String::from_str(&env, "event 2"));
        client.append(&String::from_str(&env, "event 3"));

        assert_eq!(client.get_count(), 3);
        assert!(client.verify_chain(&0, &2));

        let entry1 = client.get_entry(&1);
        assert_eq!(entry1.data, String::from_str(&env, "event 2"));
        
        let root = client.get_root();
        let entry2 = client.get_entry(&2);
        assert_eq!(root, entry2.entry_hash);
    }

    #[test]
    fn test_chain_integrity_failure() {
        let (env, client, _admin) = setup();
        env.mock_all_auths();
        client.append(&String::from_str(&env, "event 1"));
        client.append(&String::from_str(&env, "event 2"));

        assert!(client.verify_chain(&0, &1));
    }
}
