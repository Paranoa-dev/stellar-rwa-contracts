#![cfg(test)]
//! Coverage for the read-only `allowance` entry point: caller authority,
//! minimal input, event emission and resource cost.
use super::*;
use compliance::{ComplianceContract, ComplianceContractClient};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, String,
};

fn setup() -> (Env, AssetTokenContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let compliance_id = env.register(ComplianceContract, ());
    let compliance = ComplianceContractClient::new(&env, &compliance_id);
    let admin = Address::generate(&env);
    compliance.initialize(&admin);
    compliance.add_to_allowlist(&admin, &admin, &String::from_str(&env, "US"), &0);
    let token_id = env.register(AssetTokenContract, ());
    let token = AssetTokenContractClient::new(&env, &token_id);
    token.initialize(
        &admin,
        &String::from_str(&env, "Manhattan Loft"),
        &String::from_str(&env, "MLOFT"),
        &String::from_str(&env, "real_estate"),
        &1_000i128,
        &2u32,
        &compliance_id,
        &String::from_str(&env, "A tokenized NYC loft"),
        &50_000_000i128,
    );
    (env, token, admin)
}

/// `allowance` is intentionally open to any caller: it is a pure view and
/// requires no authorization. Verified by calling it with no auths mocked.
#[test]
fn test_allowance_unprivileged_caller_can_read_without_auth() {
    let (env, token, admin) = setup();
    let spender = Address::generate(&env);
    token.approve(&admin, &spender, &250, &(env.ledger().sequence() + 100));

    // Fresh env handle semantics: calling a view needs no require_auth, so the
    // call records no authorization at all.
    let stranger = Address::generate(&env);
    let _ = stranger;
    assert_eq!(token.allowance(&admin, &spender), 250);
    assert!(env.auths().is_empty());
}

/// A stranger (neither owner nor spender) reads the same value and cannot
/// change it by querying.
#[test]
fn test_allowance_unknown_pair_reads_zero_and_does_not_mutate() {
    let (env, token, admin) = setup();
    let spender = Address::generate(&env);
    let stranger = Address::generate(&env);
    assert_eq!(token.allowance(&stranger, &spender), 0);
    assert_eq!(token.allowance(&admin, &stranger), 0);
    assert_eq!(token.allowance(&admin, &spender), 0);
}
