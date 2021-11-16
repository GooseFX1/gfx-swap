use super::DerivedAccountIdentifier;
use crate::curve::{Fees, SwapCurve};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount};
use fehler::throws;
use solana_program::program_error::ProgramError;

impl DerivedAccountIdentifier for Pool {
    const IDENT: &'static [u8] = b"GFXPool";
}
#[account]
#[derive(Default, Debug)]
pub struct Pool {
    pub seed: [u8; 32],
    pub bump: u8,
    pub lp_bump: u8,
    pub admin: Pubkey,
    // sorted by token mint addresses
    pub token_mint_1: Pubkey,
    pub token_mint_2: Pubkey,
    pub token_vault_1: Pubkey,
    pub token_vault_2: Pubkey,
    pub mint: Pubkey, // the LP token mint
    pub fee_vault: Pubkey,
    pub fees: Fees,
    pub curve: SwapCurve,
    pub suspended: bool,
}

impl Pool {
    pub fn swaps<A>(&self, acc: &A) -> bool
    where
        A: Key,
    {
        acc.key() == self.token_mint_1 || acc.key() == self.token_mint_2
    }

    pub fn config(&mut self, config: &PoolConfig) {
        let PoolConfig {
            admin,
            fees,
            suspended,
        } = config;

        if let Some(admin) = admin {
            self.admin = *admin;
        }

        if let Some(fees) = fees {
            self.fees = *fees;
        }

        if let Some(suspended) = suspended {
            self.suspended = *suspended;
        }
    }
}

pub trait PoolExt<'info> {
    fn mint_lp_to(
        &self,
        mint: &Account<'info, Mint>,
        to: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;

    fn transfer_to_pool(
        &self,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        pool_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;

    fn transfer_to_user(
        &self,
        pool_ata: &Account<'info, TokenAccount>,
        user_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;

    fn transfer_lp_to_fee_vault(
        &self,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        fee_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;

    fn transfer_lp_from_fee_vault(
        &self,
        admin_ata: &Account<'info, TokenAccount>,
        fee_vault: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;

    fn burn_lp(
        &self,
        mint: &Account<'info, Mint>,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) -> Result<(), ProgramError>;
}

impl<'info> PoolExt<'info> for &'_ mut Account<'info, Pool> {
    #[throws(ProgramError)]
    fn mint_lp_to(
        &self,
        mint: &Account<'info, Mint>,
        to: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        require!(mint.key() == self.mint, WrongLPMint);

        token::mint_to(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                token::MintTo {
                    mint: mint.to_account_info(),
                    to: to.to_account_info(),
                    authority: self.to_account_info(),
                },
                &[&[Pool::IDENT, &self.seed, &[self.bump]]],
            ),
            amount,
        )?;
    }

    #[throws(ProgramError)]
    fn burn_lp(
        &self,
        mint: &Account<'info, Mint>,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        require!(mint.key() == self.mint, WrongLPMint);

        token::burn(
            CpiContext::new(
                token_program.to_account_info(),
                token::Burn {
                    mint: mint.to_account_info(),
                    to: user_ata.to_account_info(),
                    authority: user_authority.to_account_info(),
                },
            ),
            amount,
        )?;
    }

    #[throws(ProgramError)]
    fn transfer_lp_to_fee_vault(
        &self,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        fee_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        token::transfer(
            CpiContext::new(
                token_program.clone(),
                anchor_spl::token::Transfer {
                    from: user_ata.to_account_info(),
                    to: fee_ata.to_account_info(),
                    authority: user_authority.to_account_info(),
                },
            ),
            amount,
        )?;
    }

    #[throws(ProgramError)]
    fn transfer_to_pool(
        &self,
        user_authority: &AccountInfo<'info>,
        user_ata: &Account<'info, TokenAccount>,
        pool_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                token::Transfer {
                    from: user_ata.to_account_info(),
                    to: pool_ata.to_account_info(),
                    authority: user_authority.to_account_info(),
                },
            ),
            amount,
        )?;
    }

    #[throws(ProgramError)]
    fn transfer_to_user(
        &self,
        pool_ata: &Account<'info, TokenAccount>,
        user_ata: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        token::transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                token::Transfer {
                    from: pool_ata.to_account_info(),
                    to: user_ata.to_account_info(),
                    authority: self.to_account_info(),
                },
                &[&[Pool::IDENT, &self.seed, &[self.bump]]],
            ),
            amount,
        )?;
    }

    #[throws(ProgramError)]
    fn transfer_lp_from_fee_vault(
        &self,
        admin_ata: &Account<'info, TokenAccount>,
        fee_vault: &Account<'info, TokenAccount>,
        token_program: &AccountInfo<'info>,
        amount: u64,
    ) {
        token::transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                token::Transfer {
                    from: fee_vault.to_account_info(),
                    to: admin_ata.to_account_info(),
                    authority: self.to_account_info(),
                },
                &[&[Pool::IDENT, &self.seed, &[self.bump]]],
            ),
            amount,
        )?;
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct PoolConfig {
    pub admin: Option<Pubkey>,
    pub fees: Option<Fees>,
    pub suspended: Option<bool>,
}
