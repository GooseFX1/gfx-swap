use crate::curve::{CurveCalculator, Fees, SwapCurve};
use crate::states::{DerivedAccountIdentifier, LPMint, Pool};
use crate::utils::PubkeyPairExt;
use crate::LP_TOKEN_DECIMALS;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use fehler::throws;

#[derive(Accounts)]
#[instruction(seed: [u8; 32], pool_bump: u8, lp_bump: u8)]
pub struct CreatePool<'info> {
    #[account(
        init,
        seeds = [Pool::IDENT, &seed],
        bump = pool_bump,
        payer = admin,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        mint::decimals = LP_TOKEN_DECIMALS as u8,
        mint::authority = pool,
        seeds = [LPMint::IDENT, &seed],
        bump = lp_bump,
        payer = admin,
        space = Mint::LEN,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = admin,
        associated_token::mint = lp_token_mint,
        associated_token::authority = pool
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    pub token_a_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = token_a_mint,
        associated_token::authority = pool,
    )]
    pub token_a_vault: Box<Account<'info, TokenAccount>>,

    pub token_b_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = token_b_mint,
        associated_token::authority = pool,
    )]
    pub token_b_vault: Box<Account<'info, TokenAccount>>,

    pub admin: Signer<'info>, // admin account can do privileged operations

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreatePool<'info> {
    #[throws(ProgramError)]
    pub fn process(
        &mut self,
        seed: [u8; 32],
        pool_bump: u8,
        lp_bump: u8,
        fees: Fees,
        curve: SwapCurve,
    ) {
        let CreatePool {
            admin,
            pool,
            lp_token_mint,
            fee_vault: lp_token_ata_fee,
            token_a_vault,
            token_b_vault,
            token_a_mint,
            token_b_mint,
            ..
        } = self;

        fees.validate()?;
        curve.validate()?;

        let (token_a_mint, token_b_mint) = (&mut **token_a_mint, &mut **token_b_mint);
        // sort the tokens into the increasing order based on address
        let (token_vault_1, token_vault_2) =
            (token_a_vault.mint, token_b_vault.mint).sort(token_a_vault, token_b_vault)?;
        let (token_mint_1, token_mint_2) = (token_a_mint, token_b_mint).sort_self()?;

        pool.admin = admin.key();
        pool.seed = seed;
        pool.bump = pool_bump;
        pool.lp_bump = lp_bump;
        pool.token_mint_1 = token_mint_1.key();
        pool.token_mint_2 = token_mint_2.key();
        pool.token_vault_1 = token_vault_1.key();
        pool.token_vault_2 = token_vault_2.key();
        pool.mint = lp_token_mint.key();
        pool.fee_vault = lp_token_ata_fee.key();
        pool.fees = fees;
        pool.curve = curve;
    }
}
