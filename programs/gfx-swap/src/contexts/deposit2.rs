use crate::curve::{CurveCalculator, RoundDirection};
use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use crate::utils::{self, PubkeyPairExt, TupleExt};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::throws;

#[derive(Accounts)]
pub struct Deposit2<'info> {
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
        constraint = user_token_a_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&user_token_a_ata.mint) @ TokenNotSupportedByPool,
    )]
    pub user_token_a_ata: Box<Account<'info, TokenAccount>>, // let the spl program check the ownership
    #[account(
        mut,
        constraint = user_token_b_ata.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&user_token_b_ata.mint) @ TokenNotSupportedByPool,
        constraint = user_token_a_ata.mint != user_token_b_ata.mint @ SameToken,
    )]
    pub user_token_b_ata: Box<Account<'info, TokenAccount>>, // let the spl program check the ownership

    #[account(
        init_if_needed,
        associated_token::mint = lp_token_mint,
        associated_token::authority = user_wallet,
        payer = user_wallet,
    )]
    pub user_lp_ata: Box<Account<'info, TokenAccount>>,

    pub user_wallet: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Deposit2<'info> {
    #[throws(ProgramError)]
    pub fn process(
        &mut self,
        lp_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
    ) {
        let Deposit2 {
            pool,
            token_a_vault,
            token_b_vault,
            lp_token_mint,
            user_wallet,
            user_token_a_ata,
            user_token_b_ata,
            user_lp_ata,
            token_program,
            ..
        } = self;

        // sort the tokens into the increasing order based on address
        let (token1_ata_pool, token2_ata_pool) =
            (token_a_vault.mint, token_b_vault.mint).sort(token_a_vault, token_b_vault)?;
        let (maximum_token1_amount, maximum_token2_amount) =
            (user_token_a_ata.mint, user_token_b_ata.mint)
                .sort(maximum_token_a_amount, maximum_token_b_amount)?;
        let (user_token1_ata, user_token2_ata) = (user_token_a_ata.mint, user_token_b_ata.mint)
            .sort(user_token_a_ata, user_token_b_ata)?;

        let calculator = &pool.curve;
        require!(calculator.allows_deposits(), UnsupportedCurveOperation);

        let current_lp_supply = utils::to_u128(lp_token_mint.supply)?;
        let (lp_token_amount, lp_supply) = if current_lp_supply > 0 {
            (utils::to_u128(lp_token_amount)?, current_lp_supply)
        } else {
            (calculator.new_pool_supply(), calculator.new_pool_supply())
        };

        let results = calculator
            .pool_tokens_to_trading_tokens(
                lp_token_amount,
                lp_supply,
                utils::to_u128(token1_ata_pool.amount)?,
                utils::to_u128(token2_ata_pool.amount)?,
                RoundDirection::Ceiling,
            )
            .ok_or(ZeroTradingTokens)?;

        let token1_amount = utils::to_u64(results.token1_amount)?;
        require!(token1_amount < maximum_token1_amount, ExceededSlippage);
        require!(token1_amount != 0, ZeroTradingTokens);

        let token2_amount = utils::to_u64(results.token2_amount)?;
        require!(token2_amount < maximum_token2_amount, ExceededSlippage);
        require!(token2_amount != 0, ZeroTradingTokens);

        let lp_token_amount = utils::to_u64(lp_token_amount)?;

        // transfer token_a to the pool
        pool.transfer_to_pool(
            user_wallet,
            user_token1_ata,
            token1_ata_pool,
            token_program,
            token1_amount,
        )?;

        // transfer token_b to the pool
        pool.transfer_to_pool(
            user_wallet,
            user_token2_ata,
            token2_ata_pool,
            token_program,
            token2_amount,
        )?;

        // mint some lp_token to the user
        pool.mint_lp_to(lp_token_mint, user_lp_ata, token_program, lp_token_amount)?;
    }
}
