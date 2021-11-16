use crate::curve::TradeDirection;
use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use crate::utils::{self, PubkeyPairExt, TupleExt};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::{throw, throws};

#[derive(Accounts)]
pub struct Withdraw1<'info> {
    #[account(seeds = [Pool::IDENT, &pool.seed], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = token_a_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&token_a_vault.key()) @ TokenNotSupportedByPool
    )]
    pub token_a_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = token_b_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&token_b_vault.key()) @ TokenNotSupportedByPool,
        constraint = token_a_vault.mint != token_b_vault.mint @ SameToken,
    )]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [LPMint::IDENT, &pool.seed],
        bump = pool.lp_bump,
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint,
    )]
    pub lp_token_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = fee_vault.owner == pool.key() @ WrongFeeVault,
        constraint = pool.fee_vault == fee_vault.key() @ WrongFeeVault,
    )]
    pub fee_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = out_token_ata_user.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&out_token_ata_user.mint) @ TokenNotSupportedByPool
    )]
    pub out_token_ata_user: Box<Account<'info, TokenAccount>>, // let the spl program check the ownership
    #[account(
        mut,
        constraint = user_lp_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = pool.mint == user_lp_ata.mint  @ WrongLPMint
    )]
    pub user_lp_ata: Box<Account<'info, TokenAccount>>,

    pub user_wallet: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw1<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self, out_token_amount: u64, maximum_lp_token_amount: u64) {
        let Withdraw1 {
            pool,
            token_a_vault,
            token_b_vault,
            lp_token_mint,
            fee_vault: lp_token_ata_fee,
            user_wallet,
            out_token_ata_user,
            user_lp_ata,
            token_program,
        } = self;

        let (token1_ata_pool, token2_ata_pool) =
            (token_a_vault.mint, token_b_vault.mint).sort(token_a_vault, token_b_vault)?;

        let trade_direction = if out_token_ata_user.mint == pool.token_mint_1 {
            TradeDirection::AtoB
        } else if out_token_ata_user.mint == pool.token_mint_2 {
            TradeDirection::BtoA
        } else {
            throw!(IncorrectSwapAccount);
        };

        let lp_token_supply = utils::to_u128(lp_token_mint.supply)?;
        let pool_token1_amount = utils::to_u128(token1_ata_pool.amount)?;
        let pool_token2_amount = utils::to_u128(token2_ata_pool.amount)?;

        let burn_pool_token_amount = pool
            .curve
            .withdraw_single_token_type_exact_out(
                utils::to_u128(out_token_amount)?,
                pool_token1_amount,
                pool_token2_amount,
                lp_token_supply,
                trade_direction,
                &pool.fees,
            )
            .ok_or(ZeroTradingTokens)?;

        let withdraw_fee: u128 = pool
            .fees
            .owner_withdraw_fee(burn_pool_token_amount)
            .ok_or(FeeCalculationFailure)?;

        let lp_token_amount = burn_pool_token_amount
            .checked_add(withdraw_fee)
            .ok_or(CalculationFailure)?;

        if utils::to_u64(lp_token_amount)? > maximum_lp_token_amount {
            throw!(ExceededSlippage);
        }
        if lp_token_amount == 0 {
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
            utils::to_u64(burn_pool_token_amount)?,
        )?;

        match trade_direction {
            TradeDirection::AtoB => {
                pool.transfer_to_user(
                    token1_ata_pool,
                    out_token_ata_user,
                    token_program,
                    out_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                pool.transfer_to_user(
                    token2_ata_pool,
                    out_token_ata_user,
                    token_program,
                    out_token_amount,
                )?;
            }
        }
    }
}
