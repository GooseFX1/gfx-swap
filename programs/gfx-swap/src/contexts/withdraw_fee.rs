use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::throws;

#[derive(Accounts)]
pub struct WithdrawFee<'info> {
    #[account(
        seeds = [Pool::IDENT, &pool.seed],
        bump = pool.bump,
        has_one = admin @ WrongAdmin
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        seeds = [LPMint::IDENT, &pool.seed],
        bump = pool.lp_bump,
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::authority = pool,
        associated_token::mint = lp_token_mint,
        constraint = pool.fee_vault == fee_vault.key() @ WrongFeeVault,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = lp_token_mint,
        associated_token::authority = admin,
    )]
    pub admin_ata: Box<Account<'info, TokenAccount>>,

    pub admin: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawFee<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self) {
        let WithdrawFee {
            pool,
            admin_ata,
            fee_vault,
            token_program,
            ..
        } = self;

        pool.transfer_lp_from_fee_vault(admin_ata, fee_vault, token_program, fee_vault.amount)?;
    }
}
