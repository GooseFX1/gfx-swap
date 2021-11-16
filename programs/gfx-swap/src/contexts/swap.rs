use crate::curve::TradeDirection;
use crate::errors::ErrorCode::*;
use crate::states::{DerivedAccountIdentifier, LPMint, Pool, PoolExt};
use crate::utils::{self, TupleExt};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use fehler::{throw, throws};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(seeds = [Pool::IDENT, &pool.seed], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = in_token_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&in_token_vault.key()) @ TokenNotSupportedByPool,
    )]
    pub in_token_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = out_token_vault.owner == pool.key() @ WrongATAOwner,
        constraint = (pool.token_vault_1, pool.token_vault_2).contains(&out_token_vault.key()) @ TokenNotSupportedByPool,
        constraint = in_token_vault.mint != out_token_vault.mint @ SameToken,
    )]
    pub out_token_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [LPMint::IDENT, &pool.seed],
        bump = pool.lp_bump,
        constraint = pool.mint == lp_token_mint.key() @ WrongLPMint,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        constraint = fee_vault.owner == pool.key() @ WrongFeeVault,
        constraint = pool.fee_vault == fee_vault.key() @ WrongFeeVault,
    )]
    pub fee_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = in_token_ata_user.owner == user_wallet.key() @ WrongATAOwner,
        constraint =  (pool.token_mint_1, pool.token_mint_2).contains(&in_token_ata_user.mint) @ TokenNotSupportedByPool,
    )]
    pub in_token_ata_user: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = out_token_ata_user.owner == user_wallet.key() @ WrongATAOwner,
        constraint = (pool.token_mint_1, pool.token_mint_2).contains(&out_token_ata_user.mint) @ TokenNotSupportedByPool,
        constraint = in_token_ata_user.mint != out_token_ata_user.mint @ SameToken,
    )]
    pub out_token_ata_user: Box<Account<'info, TokenAccount>>,

    pub user_wallet: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self, amount_in: u64, minimum_amount_out: u64) {
        let Swap {
            pool,
            in_token_vault: in_token_ata_pool,
            out_token_vault: out_token_ata_pool,
            lp_token_mint,
            fee_vault: lp_token_ata_fee,
            user_wallet,
            in_token_ata_user,
            out_token_ata_user,
            token_program,
        } = self;

        let trade_direction = if in_token_ata_user.mint == pool.token_mint_1 {
            TradeDirection::AtoB
        } else if in_token_ata_user.mint == pool.token_mint_2 {
            TradeDirection::BtoA
        } else {
            throw!(IncorrectSwapAccount);
        };

        let result = pool
            .curve
            .swap(
                utils::to_u128(amount_in)?,
                utils::to_u128(in_token_ata_pool.amount)?,
                utils::to_u128(out_token_ata_pool.amount)?,
                trade_direction,
                &pool.fees,
            )
            .ok_or(ZeroTradingTokens)?;
        if result.destination_amount_swapped < utils::to_u128(minimum_amount_out)? {
            throw!(ExceededSlippage);
        }

        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (
                result.new_swap_source_amount,
                result.new_swap_destination_amount,
            ),
            TradeDirection::BtoA => (
                result.new_swap_destination_amount,
                result.new_swap_source_amount,
            ),
        };

        // transfer token_src to the pool
        pool.transfer_to_pool(
            user_wallet,
            in_token_ata_user,
            in_token_ata_pool,
            token_program,
            utils::to_u64(result.source_amount_swapped)?,
        )?;

        // transfer token_dst to the user
        pool.transfer_to_user(
            out_token_ata_pool,
            out_token_ata_user,
            token_program,
            utils::to_u64(result.destination_amount_swapped)?,
        )?;

        // trading fees

        let lp_token_amount = pool
            .curve
            .withdraw_single_token_type_exact_out(
                result.owner_fee,
                swap_token_a_amount,
                swap_token_b_amount,
                utils::to_u128(lp_token_mint.supply)?,
                trade_direction,
                &pool.fees,
            )
            .ok_or(FeeCalculationFailure)?;

        if lp_token_amount > 0 {
            // Allow error to fall through

            // transfer some fee to the host
            // if let Ok(host_fee_account_info) = next_account_info(account_info_iter) {
            //     let host_fee_account = Self::unpack_token_account(
            //         host_fee_account_info,
            //         token_swap.token_program_id(),
            //     )?;
            //     if *pool_mint_info.key != host_fee_account.mint {
            //         return Err(SwapError::IncorrectPoolMint.into());
            //     }
            //     let host_fee = token_swap
            //         .fees()
            //         .host_fee(pool_token_amount)
            //         .ok_or(SwapError::FeeCalculationFailure)?;
            //     if host_fee > 0 {
            //         pool_token_amount = pool_token_amount
            //             .checked_sub(host_fee)
            //             .ok_or(SwapError::FeeCalculationFailure)?;
            //         Self::token_mint_to(
            //             swap_info.key,
            //             token_program_info.clone(),
            //             pool_mint_info.clone(),
            //             host_fee_account_info.clone(),
            //             authority_info.clone(),
            //             token_swap.nonce(),
            //             to_u64(host_fee)?,
            //         )?;
            //     }
            // }

            pool.mint_lp_to(
                lp_token_mint,
                lp_token_ata_fee,
                token_program,
                utils::to_u64(lp_token_amount)?,
            )?;
        }
    }
}
