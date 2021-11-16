use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::throws;

#[derive(Accounts)]
pub struct MintLPTo<'info> {
    #[account(seeds = [Pool::IDENT, &pool.seed], bump = pool.bump, has_one = admin @ WrongAdmin)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [LPMint::IDENT, &pool.seed],
        bump = pool.lp_bump,
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint
    )]
    pub lp_token_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        associated_token::mint = lp_token_mint,
        associated_token::authority = admin,
        payer = admin,
    )]
    pub recipient_ata: Account<'info, TokenAccount>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> MintLPTo<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self, amount: u64) {
        let Self {
            pool,
            lp_token_mint,
            recipient_ata,
            token_program,
            ..
        } = self;
        pool.mint_lp_to(lp_token_mint, recipient_ata, token_program, amount)?;
    }
}
