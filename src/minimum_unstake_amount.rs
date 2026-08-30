//! Admin-configurable minimum unstake amount (issue #441).
//!
//! Prevents tiny dust withdrawals that waste ledger space and create
//! accounting noise. Users must unstake at least the configured minimum per
//! transaction, or unstake their full remaining balance — a full position
//! exit is always allowed so users can never be trapped.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so this uses raw `Symbol`-keyed
//! storage, matching `balance.rs`.

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol};

use crate::balance;
use crate::errors::VaultError;
use crate::vault::VaultContract;

const MIN_UNSTAKE_KEY: Symbol = symbol_short!("mn_unstk");

/// Enforce the minimum-unstake rule for an unstake of `amount` against the
/// caller's full position. Full position exit is always allowed.
///
/// Returns `Ok(())` when the amount is above the configured minimum, is a
/// full position exit, or the minimum is disabled (0).
pub fn enforce_min_unstake(
    env: &Env,
    amount: i128,
    position_amount: i128,
) -> Result<(), VaultError> {
    let min = get_min_unstake_amount(env);
    if min > 0 && amount < min && amount != position_amount {
        return Err(VaultError::BelowMinimumUnstake);
    }
    Ok(())
}

/// Read the configured minimum unstake amount (0 = disabled).
pub fn get_min_unstake_amount(env: &Env) -> i128 {
    env.storage().instance().get(&MIN_UNSTAKE_KEY).unwrap_or(0)
}

fn set_min_unstake_amount(env: &Env, amount: i128) {
    env.storage().instance().set(&MIN_UNSTAKE_KEY, &amount);
}

#[contractimpl]
impl VaultContract {
    /// Configure the minimum unstake amount. Admin only. `0` disables the
    /// check (the default).
    pub fn set_min_unstake_amount(
        env: Env,
        admin: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        admin.require_auth();
        crate::admin::require_admin(&env)?;

        if amount < 0 {
            return Err(VaultError::InvalidRate);
        }
        set_min_unstake_amount(&env, amount);
        Ok(())
    }

    /// Read-only query: the configured minimum unstake amount (0 = disabled).
    pub fn get_min_unstake_amount(env: Env) -> i128 {
        get_min_unstake_amount(&env)
    }

    /// Helper: the nearest valid unstake amount for a requested `amount`.
    /// Rounds up to the configured minimum when below it; a full position
    /// exit is always a valid value.
    pub fn get_nearest_valid_unstake(env: Env, amount: i128) -> i128 {
        let min = get_min_unstake_amount(&env);
        if min > 0 && amount < min {
            min
        } else {
            amount
        }
    }

    /// Read-only query: the caller's nearest valid unstake amount given
    /// their current full position, via `get_nearest_valid_unstake`.
    pub fn nearest_valid_unstake_for(env: Env, user: Address) -> i128 {
        let position_amount = balance::get_shares(&env, &user);
        Self::get_nearest_valid_unstake(env, position_amount)
    }
}
