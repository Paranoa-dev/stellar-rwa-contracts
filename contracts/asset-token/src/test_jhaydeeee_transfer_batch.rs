//! Edge-case tests for `AssetTokenContract::transfer_batch`
//! (zero/empty input and unprivileged callers).
#![cfg(test)]
use super::*;
use compliance::{ComplianceContract, ComplianceContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

struct Ctx {
    env: Env,
    token: AssetTokenContractClient<'static>,
    compliance: ComplianceContractClient<'static>,
    admin: Address,
}

fn ctx(supply: i128) -> Ctx {
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
        &supply,
        &2u32,
        &compliance_id,
        &String::from_str(&env, "A tokenized NYC loft"),
        &50_000_000i128,
    );
    Ctx {
        env,
        token,
        compliance,
        admin,
    }
}

fn approved(c: &Ctx) -> Address {
    let a = Address::generate(&c.env);
    c.compliance
        .add_to_allowlist(&c.admin, &a, &String::from_str(&c.env, "US"), &0);
    a
}

// ---- issue #527: zero / empty input ----

/// An empty batch is deliberately a no-op: the sender is still authenticated,
/// pause-checked and compliance-checked, but no balances change.
#[test]
fn test_transfer_batch_empty_list_is_noop() {
    let c = ctx(1_000);
    let bob = approved(&c);
    let batch: Vec<(Address, i128)> = Vec::new(&c.env);
    c.token.transfer_batch(&c.admin, &batch);
    assert_eq!(c.token.balance(&c.admin), 1_000);
    assert_eq!(c.token.balance(&bob), 0);
}

/// The sender gate runs before the loop, so even an empty batch from a
/// non-compliant sender reverts with `SenderNotCompliant` (#7).
#[test]
fn test_transfer_batch_empty_list_noncompliant_sender_rejected() {
    let c = ctx(1_000);
    let stranger = Address::generate(&c.env);
    let batch: Vec<(Address, i128)> = Vec::new(&c.env);
    let res = c.token.try_transfer_batch(&stranger, &batch);
    assert!(res.is_err());
}

/// An empty batch is also rejected while the token is paused (#6).
#[test]
fn test_transfer_batch_empty_list_rejected_when_paused() {
    let c = ctx(1_000);
    c.token.pause(&c.admin);
    let batch: Vec<(Address, i128)> = Vec::new(&c.env);
    assert!(c.token.try_transfer_batch(&c.admin, &batch).is_err());
}

/// A zero amount is rejected with `InvalidAmount` (#5), matching `transfer`.
#[test]
fn test_transfer_batch_zero_amount_rejected() {
    let c = ctx(1_000);
    let bob = approved(&c);
    let mut batch: Vec<(Address, i128)> = Vec::new(&c.env);
    batch.push_back((bob.clone(), 0));
    assert!(c.token.try_transfer_batch(&c.admin, &batch).is_err());
    assert_eq!(c.token.balance(&c.admin), 1_000);
    assert_eq!(c.token.balance(&bob), 0);
}

/// A zero entry after a valid one reverts the whole batch atomically.
#[test]
fn test_transfer_batch_zero_entry_reverts_earlier_entries() {
    let c = ctx(1_000);
    let bob = approved(&c);
    let carol = approved(&c);
    let mut batch: Vec<(Address, i128)> = Vec::new(&c.env);
    batch.push_back((bob.clone(), 100));
    batch.push_back((carol, 0));
    assert!(c.token.try_transfer_batch(&c.admin, &batch).is_err());
    assert_eq!(c.token.balance(&c.admin), 1_000);
    assert_eq!(c.token.balance(&bob), 0);
}

/// A negative amount is rejected like zero.
#[test]
fn test_transfer_batch_negative_amount_rejected() {
    let c = ctx(1_000);
    let bob = approved(&c);
    let mut batch: Vec<(Address, i128)> = Vec::new(&c.env);
    batch.push_back((bob, -1));
    assert!(c.token.try_transfer_batch(&c.admin, &batch).is_err());
}
