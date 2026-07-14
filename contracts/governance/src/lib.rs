#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, IntoVal, String,
    Symbol, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub new_impl: BytesN<32>,
    pub scheduled_at: u64,
    pub approvals: Vec<Address>,
    pub executed: bool,
}

#[contract]
pub struct Governance;

const ADMINS: Symbol = symbol_short!("admins");
const TIMELOCK: Symbol = symbol_short!("timelock");
const PROPOSAL_COUNT: Symbol = symbol_short!("prop_cnt");
const AUDIT_LOG: Symbol = symbol_short!("audit_log");

#[contractimpl]
impl Governance {
    pub fn initialize(env: Env, admins: Vec<Address>, timelock_secs: u64, audit_log: Address) {
        if env.storage().instance().has(&ADMINS) {
            panic!("already initialized");
        }
        if admins.len() < 3 {
            panic!("min 3 admins required");
        }
        env.storage().instance().set(&ADMINS, &admins);
        env.storage().instance().set(&TIMELOCK, &timelock_secs);
        env.storage().instance().set(&AUDIT_LOG, &audit_log);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
    }

    pub fn propose_upgrade(env: Env, proposer: Address, new_impl: BytesN<32>) -> u32 {
        let admins: Vec<Address> = env
            .storage()
            .instance()
            .get(&ADMINS)
            .expect("not initialized");
        if !admins.contains(&proposer) {
            panic!("not an admin");
        }
        proposer.require_auth();

        let count: u32 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        let timelock: u64 = env.storage().instance().get(&TIMELOCK).unwrap_or(172800);

        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());

        let proposal = Proposal {
            new_impl: new_impl.clone(),
            scheduled_at: env.ledger().timestamp() + timelock,
            approvals,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&(symbol_short!("prop"), count), &proposal);
        env.storage().instance().set(&PROPOSAL_COUNT, &(count + 1));

        env.events()
            .publish((symbol_short!("upg_prop"), count), new_impl);

        count
    }

    pub fn approve(env: Env, admin: Address, proposal_id: u32) {
        let admins: Vec<Address> = env
            .storage()
            .instance()
            .get(&ADMINS)
            .expect("not initialized");
        if !admins.contains(&admin) {
            panic!("not an admin");
        }
        admin.require_auth();

        let key = (symbol_short!("prop"), proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .expect("proposal not found");

        if proposal.executed {
            panic!("already executed");
        }

        if proposal.approvals.contains(&admin) {
            panic!("already approved");
        }

        proposal.approvals.push_back(admin);
        env.storage().persistent().set(&key, &proposal);
    }

    pub fn execute_upgrade(env: Env, proposal_id: u32) {
        let key = (symbol_short!("prop"), proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .expect("proposal not found");

        if proposal.executed {
            panic!("already executed");
        }

        if proposal.approvals.len() < 3 {
            // Requirement: min 3 approvals (matches min 3 admins)
            panic!("insufficient approvals");
        }

        if env.ledger().timestamp() < proposal.scheduled_at {
            panic!("timelock not expired");
        }

        env.deployer()
            .update_current_contract_wasm(proposal.new_impl.clone());

        proposal.executed = true;
        env.storage().persistent().set(&key, &proposal);

        env.events()
            .publish((symbol_short!("upg_exec"), proposal_id), proposal.new_impl);
    }

    pub fn object(env: Env, objector: Address, reason: String) {
        objector.require_auth();

        let audit_log_addr: Address = env
            .storage()
            .instance()
            .get(&AUDIT_LOG)
            .expect("not initialized");

        // Log the objection to AuditLog
        env.invoke_contract::<u32>(
            &audit_log_addr,
            &symbol_short!("append"),
            (reason,).into_val(&env),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec};

    fn setup() -> (Env, GovernanceClient<'static>, Vec<Address>, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);

        let mut admins = Vec::new(&env);
        admins.push_back(Address::generate(&env));
        admins.push_back(Address::generate(&env));
        admins.push_back(Address::generate(&env));

        let audit_log = Address::generate(&env);
        client.initialize(&admins, &100, &audit_log);

        (env, client, admins, audit_log)
    }

    #[test]
    fn test_propose_and_approve() {
        let (env, client, admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let proposer = admins.get(0).unwrap();

        env.mock_all_auths();
        let prop_id = client.propose_upgrade(&proposer, &new_impl);

        client.approve(&admins.get(1).unwrap(), &prop_id);
        client.approve(&admins.get(2).unwrap(), &prop_id);
    }

    #[test]
    #[should_panic(expected = "insufficient approvals")]
    fn test_execute_insufficient_approvals() {
        let (env, client, admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let proposer = admins.get(0).unwrap();

        env.mock_all_auths();
        let prop_id = client.propose_upgrade(&proposer, &new_impl);

        client.execute_upgrade(&prop_id);
    }

    #[test]
    #[should_panic(expected = "not an admin")]
    fn test_propose_by_non_admin() {
        let (env, client, _admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let non_admin = Address::generate(&env);

        env.mock_all_auths();
        client.propose_upgrade(&non_admin, &new_impl);
    }

    #[test]
    #[should_panic(expected = "already approved")]
    fn test_duplicate_approval_rejected() {
        let (env, client, admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let proposer = admins.get(0).unwrap();

        env.mock_all_auths();
        let prop_id = client.propose_upgrade(&proposer, &new_impl);

        client.approve(&proposer, &prop_id);
    }

    #[test]
    #[should_panic(expected = "timelock not expired")]
    fn test_execute_timelock_not_expired() {
        let (env, client, admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let proposer = admins.get(0).unwrap();

        env.mock_all_auths();
        let prop_id = client.propose_upgrade(&proposer, &new_impl);
        client.approve(&admins.get(1).unwrap(), &prop_id);
        client.approve(&admins.get(2).unwrap(), &prop_id);

        client.execute_upgrade(&prop_id);
    }

    #[test]
    #[should_panic(expected = "not an admin")]
    fn test_approve_by_non_admin() {
        let (env, client, admins, _audit_log) = setup();

        let new_impl = BytesN::from_array(&env, &[1u8; 32]);
        let proposer = admins.get(0).unwrap();
        let non_admin = Address::generate(&env);

        env.mock_all_auths();
        let prop_id = client.propose_upgrade(&proposer, &new_impl);

        client.approve(&non_admin, &prop_id);
    }
}
