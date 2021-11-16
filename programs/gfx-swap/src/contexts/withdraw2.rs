use crate::curve::{CurveCalculator, RoundDirection};
use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use crate::utils::{self, PubkeyPairExt, TupleExt};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::{throw, throws};

#[derive(Accounts)]
pub struct Withdraw2<'info> {
    #[account(seeds = [Pool::IDENT, &pool.seed], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = token_a_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&token_a_vault.key()) @ TokenNotSupportedByPool
    )]
    pub token_a_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = token_b_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&token_b_vault.key()) @ TokenNotSupportedByPool,
        constraint = token_a_vault.mint != token_b_vault.mint @ SameToken,
    )]
    pub token_b_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [LPMint::IDENT, &pool.seed],
        bump = pool.lp_bump,
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        constraint = fee_vault.owner == pool.key() @ WrongATAOwner,
       constraint = pool.fee_vault == fee_vault.key() @ WrongFeeVault,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_token_a_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&user_token_a_ata.mint) @ TokenNotSupportedByPool,
    )]
    pub user_token_a_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = user_token_b_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&user_token_b_ata.mint) @ TokenNotSupportedByPool,
        constraint = user_token_a_ata.mint != user_token_b_ata.mint @ SameToken,
    )]
    pub user_token_b_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = user_lp_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = pool.mint == user_lp_ata.mint @ WrongLPMint
    )]
    pub user_lp_ata: Box<Account<'info, TokenAccount>>,

    pub user_wallet: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw2<'info> {
    #[throws(ProgramError)]
    pub fn process(
        &mut self,
        lp_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
    ) {
        let Withdraw2 {
            pool,
            token_a_vault,
            token_b_vault,
            lp_token_mint,
            fee_vault: lp_token_ata_fee,
            user_wallet,
            user_token_a_ata,
            user_token_b_ata,
            user_lp_ata,
            token_program,
        } = self;

        // sort the tokens into the increasing order based on address
        let (token1_ata_pool, token2_ata_pool) =
            (token_a_vault.mint, token_b_vault.mint).sort(token_a_vault, token_b_vault)?;
        let (minimum_token1_amount, minimum_token2_amount) =
            (user_token_a_ata.mint, user_token_b_ata.mint)
                .sort(minimum_token_a_amount, minimum_token_b_amount)?;
        let (user_token1_ata, user_token2_ata) = (user_token_a_ata.mint, user_token_b_ata.mint)
            .sort(user_token_a_ata, user_token_b_ata)?;

        let calculator = &pool.curve;

        let withdraw_fee: u128 = pool
            .fees
            .owner_withdraw_fee(utils::to_u128(lp_token_amount)?)
            .ok_or(FeeCalculationFailure)?;

        let lp_token_amount = utils::to_u128(lp_token_amount)?
            .checked_sub(withdraw_fee)
            .ok_or(CalculationFailure)?;

        let results = calculator
            .pool_tokens_to_trading_tokens(
                lp_token_amount,
                utils::to_u128(lp_token_mint.supply)?,
                utils::to_u128(token1_ata_pool.amount)?,
                utils::to_u128(token2_ata_pool.amount)?,
                RoundDirection::Floor,
            )
            .ok_or(ZeroTradingTokens)?;

        let token1_amount = utils::to_u64(results.token1_amount)?;
        let token1_amount = std::cmp::min(token1_ata_pool.amount, token1_amount);
        if token1_amount < minimum_token1_amount {
            throw!(ExceededSlippage);
        }
        if token1_amount == 0 && token1_ata_pool.amount != 0 {
            throw!(ZeroTradingTokens);
        }
        let token2_amount = utils::to_u64(results.token2_amount)?;
        let token2_amount = std::cmp::min(token2_ata_pool.amount, token2_amount);
        if token2_amount < minimum_token2_amount {
            throw!(ExceededSlippage);
        }
        if token2_amount == 0 && token2_ata_pool.amount != 0 {
            throw!(ZeroTradingTokens);
        }

        if withdraw_fee > 0 {
            pool.transfer_lp_to_fee_vault(
                user_wallet,
                user_lp_ata,
                lp_token_ata_fee,
                token_program,
                utils::to_u64(withdraw_fee)?,
            )?;
        }

        pool.burn_lp(
            lp_token_mint,
            user_wallet,
            user_lp_ata,
            token_program,
            utils::to_u64(lp_token_amount)?,
        )?;

        if token1_amount > 0 {
            pool.transfer_to_user(
                token1_ata_pool,
                user_token1_ata,
                token_program,
                token1_amount,
            )?;
        }
        if token2_amount > 0 {
            pool.transfer_to_user(
                token2_ata_pool,
                user_token2_ata,
                token_program,
                token1_amount,
            )?;
        }
    }
}
