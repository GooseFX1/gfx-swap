// # Account Checks
//
// There are two ways to check the if the accounts passed in are valid: through the `#[account]` attribute
// and the `#[access_control]` attribute. The former should be avoided because it will only report a
// contraint violated error to the program caller but the latter supports customized error message,
// which is more informative to the program caller.
// However, `#[access_control]` is specified in the `#[program]` section which is a little
// bit far away from the definition of the accounts. Extra care should be taken when writing these checks.
//
// # Token Orders
//
// For a pool it supports 2 tokens, sorted and written down in the token_mints attribute.
// However, we don't force the order for these two token accounts when calling our program.
// Instead, we will sort them internally. To distinguish sorted and unsorted token accounts,
// the unsorted ones have `token_a`, `token_b` in their names and the sorted ones have `token1`, `token2`
// in the names.

mod constraints;
mod contexts;
mod curve;
mod errors;
mod states;
mod utils;

pub use contexts::*;
pub use curve::{
    ConstantProductCurve, CurveCalculator, Fees, RoundDirection, StableCurve, SwapCurve,
    TradeDirection,
};
pub use errors::ErrorCode;
pub use program_id::*;
pub use states::{DerivedAccountIdentifier, LPMint, Pool, PoolConfig};

use anchor_lang::prelude::*;
use constraints::suspended;
use fehler::throws;

#[cfg(not(feature = "ci"))]
mod program_id {
    use anchor_lang::prelude::*;
    declare_id!("5UfYq7dZscctLgomBj51ucx1D3hLdGXYdskKqJBir9FF");
}

#[cfg(feature = "ci")]
include!(concat!(env!("OUT_DIR"), "/program_id.rs"));

pub const LP_TOKEN_DECIMALS: u8 = 9;

#[program]
pub mod pool {
    use super::*;

    // ========== User Instructions ==========

    // Input: I want to get `lp_token_amount` of lp_tokens
    // Action: Deduct corresponding amount of token a and token b from user's ata account
    // Constraint: the deducted amount of token a and token b cannot exceed `maximum_token_a_amount` and `maximum_token_b_amount`
    #[throws(ProgramError)]
    #[access_control(suspended(&ctx.accounts.pool))]
    pub fn deposit2(
        ctx: Context<Deposit2>,
        lp_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
    ) {
        ctx.accounts.process(
            lp_token_amount,
            maximum_token_a_amount,
            maximum_token_b_amount,
        )?
    }

    #[throws(ProgramError)]
    #[access_control(suspended(&ctx.accounts.pool))]
    pub fn withdraw2(
        ctx: Context<Withdraw2>,
        lp_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
    ) {
        ctx.accounts.process(
            lp_token_amount,
            minimum_token_a_amount,
            minimum_token_b_amount,
        )?
    }

    #[throws(ProgramError)]
    #[access_control(suspended(&ctx.accounts.pool))]
    pub fn deposit1(ctx: Context<Deposit1>, in_token_amount: u64, minimum_pool_token_amount: u64) {
        ctx.accounts
            .process(in_token_amount, minimum_pool_token_amount)?
    }

    #[throws(ProgramError)]
    #[access_control(suspended(&ctx.accounts.pool))]
    pub fn withdraw1(ctx: Context<Withdraw1>, out_token_amount: u64, maximum_lp_token_amount: u64) {
        ctx.accounts
            .process(out_token_amount, maximum_lp_token_amount)?
    }

    #[throws(ProgramError)]
    #[access_control(suspended(&ctx.accounts.pool))]
    pub fn swap(ctx: Context<Swap>, amount_in: u64, minimum_amount_out: u64) {
        ctx.accounts.process(amount_in, minimum_amount_out)?
    }

    // ========== Admin Instructions ==========

    #[throws(ProgramError)]
    pub fn create_pool(
        ctx: Context<CreatePool>,
        seed: [u8; 32],
        pool_bump: u8,
        lp_bump: u8,
        fees: Fees,
        swap_curve: SwapCurve,
    ) {
        ctx.accounts
            .process(seed, pool_bump, lp_bump, fees, swap_curve)?
    }

    #[throws(ProgramError)]
    pub fn mint_lp_to(ctx: Context<MintLPTo>, n: u64) {
        ctx.accounts.process(n)?
    }

    #[throws(ProgramError)]
    pub fn withdraw_fee(ctx: Context<WithdrawFee>) {
        ctx.accounts.process()?
    }

    #[throws(ProgramError)]
    pub fn config_pool(ctx: Context<ConfigPool>, config: PoolConfig) {
        ctx.accounts.process(&config)?
    }
}
