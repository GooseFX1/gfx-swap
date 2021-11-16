use crate::curve::{CurveCalculator, TradeDirection};
use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use crate::utils::{self, PubkeyPairExt, TupleExt};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::{throw, throws};

#[derive(Accounts)]
pub struct Deposit1<'info> {
    #[account(seeds = [Pool::IDENT, &pool.seed], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = token_a_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&token_a_vault.key()) @ TokenNotSupportedByPool,
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
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        constraint = in_token_ata_user.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&in_token_ata_user.mint) @ TokenNotSupportedByPool,
    )]
    pub in_token_ata_user: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        associated_token::mint = lp_token_mint,
        associated_token::authority = user_wallet,
        payer = user_wallet,
    )]
    pub user_lp_ata: Box<Account<'info, TokenAccount>>,

    pub user_wallet: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Deposit1<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self, in_token_amount: u64, minimum_pool_token_amount: u64) {
        let Deposit1 {
            pool,
            token_a_vault,
            token_b_vault,
            lp_token_mint,
            user_wallet,
            in_token_ata_user,
            user_lp_ata,
            token_program,
            ..
        } = self;
        let (token1_ata_pool, token2_ata_pool) =
            (token_a_vault.mint, token_b_vault.mint).sort(token_a_vault, token_b_vault)?;

        let trade_direction = if in_token_ata_user.mint == pool.token_mint_1 {
            TradeDirection::AtoB
        } else if in_token_ata_user.mint == pool.token_mint_2 {
            TradeDirection::BtoA
        } else {
            throw!(IncorrectSwapAccount);
        };

        let lp_token_supply = utils::to_u128(lp_token_mint.supply)?;
        let lp_token_amount = if lp_token_supply > 0 {
            pool.curve
                .deposit_single_token_type(
                    utils::to_u128(in_token_amount)?,
                    utils::to_u128(token1_ata_pool.amount)?,
                    utils::to_u128(token2_ata_pool.amount)?,
                    lp_token_supply,
                    trade_direction,
                    &pool.fees,
                )
                .ok_or(ZeroTradingTokens)?
        } else {
            pool.curve.new_pool_supply()
        };

        let lp_token_amount = utils::to_u64(lp_token_amount)?;
        if lp_token_amount < minimum_pool_token_amount {
            throw!(ExceededSlippage);
        }
        if lp_token_amount == 0 {
            throw!(ZeroTradingTokens);
        }

        match trade_direction {
            TradeDirection::AtoB => {
                pool.transfer_to_pool(
                    user_wallet,
                    in_token_ata_user,
                    token1_ata_pool,
                    token_program,
                    in_token_amount,
                )?;
            }
            TradeDirection::BtoA => {
                pool.transfer_to_pool(
                    user_wallet,
                    in_token_ata_user,
                    token2_ata_pool,
                    token_program,
                    in_token_amount,
                )?;
            }
        }

        pool.mint_lp_to(lp_token_mint, user_lp_ata, token_program, lp_token_amount)?;
    }
}
