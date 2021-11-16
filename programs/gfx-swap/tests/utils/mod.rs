#![allow(dead_code)]

use anchor_client::Program;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use anyhow::Error;
use fehler::throws;
use gfx_solana_utils::{AnchorClientErrorExt, ApplyDecimal};
use gfx_swap::{ConstantProductCurve, ErrorCode, Fees, SwapCurve};
use num_traits::AsPrimitive;
use solana_sdk::{
    signature::Keypair, signature::Signer, system_program, sysvar, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

#[throws(Error)]
pub fn create_pool_impl(
    program: &Program,
    pool: Pubkey,
    lp_mint: Pubkey,
    token_a: Pubkey,
    token_b: Pubkey,
    admin: &Keypair,
    seed: [u8; 32],
    pool_bump: u8,
    lp_bump: u8,

    bootstrap: bool,
) {
    let tx = program
        .request()
        .accounts(gfx_swap::accounts::CreatePool {
            pool,

            lp_token_mint: lp_mint,
            fee_vault: get_associated_token_address(&pool, &lp_mint),

            token_a_mint: token_a,
            token_a_vault: get_associated_token_address(&pool, &token_a),

            token_b_mint: token_b,
            token_b_vault: get_associated_token_address(&pool, &token_b),

            admin: admin.pubkey(),

            associated_token_program: spl_associated_token_account::id(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
        })
        .args(gfx_swap::instruction::CreatePool {
            seed,
            lp_bump,
            pool_bump,
            fees: Fees {
                trade_fee_numerator: 1,
                trade_fee_denominator: 1000, // 0.1% trading fee
                owner_trade_fee_numerator: 1,
                owner_trade_fee_denominator: 10000, // 0.01% trading fee to us
                owner_withdraw_fee_numerator: 1,
                owner_withdraw_fee_denominator: 10000, // 0.01% withdraw fee to us
                host_fee_numerator: 0,
                host_fee_denominator: 0,
            },
            swap_curve: SwapCurve::ConstantProductCurve(ConstantProductCurve::new()),
        })
        .signer(admin)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "CreatePool: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );

    if bootstrap {
        // bootstrap the pool, set 1 token_a = 1 token_b = 1 lp token

        let tx = program
            .request()
            .accounts(gfx_swap::accounts::MintLPTo {
                admin: admin.pubkey(),
                pool,
                recipient_ata: get_associated_token_address(&admin.pubkey(), &lp_mint),
                lp_token_mint: lp_mint,
                token_program: spl_token::id(),
                system_program: system_program::id(),
                rent: sysvar::rent::id(),
                associated_token_program: spl_associated_token_account::id(),
            })
            .args(gfx_swap::instruction::MintLpTo { n: 1 })
            .signer(admin)
            .send()
            .map_err(|e| e.canonicalize::<ErrorCode>())?;
        println!(
            "MintLpTo: https://explorer.solana.com/tx/{}?cluster=devnet",
            tx
        );

        let rpc_client = program.rpc();
        let tx = rpc_client.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
            &[
                spl_token::instruction::transfer(
                    &spl_token::id(),
                    &get_associated_token_address(&admin.pubkey(), &token_a),
                    &get_associated_token_address(&pool, &token_a),
                    &admin.pubkey(),
                    &[],
                    1,
                )?,
                spl_token::instruction::transfer(
                    &spl_token::id(),
                    &get_associated_token_address(&admin.pubkey(), &token_b),
                    &get_associated_token_address(&pool, &token_b),
                    &admin.pubkey(),
                    &[],
                    1,
                )?,
            ],
            Some(&admin.pubkey()),
            &[admin],
            rpc_client.get_recent_blockhash()?.0,
        ))?;

        println!(
            "Bootstrap Pool: https://explorer.solana.com/tx/{}?cluster=devnet",
            tx
        );
    }
}

#[throws(Error)]
pub fn deposit2_impl(
    program: &Program,
    pool: Pubkey,
    lp_mint: Pubkey,
    token_a: Pubkey,
    token_b: Pubkey,
    user: &Keypair,
    lp_amount: f64,
) {
    let amint: Mint = program.account(token_a)?;
    let bmint: Mint = program.account(token_b)?;
    let lpmint: Mint = program.account(lp_mint)?;

    let tx = program
        .request()
        .accounts(gfx_swap::accounts::Deposit2 {
            pool: pool,
            token_a_vault: get_associated_token_address(&pool, &token_a),
            token_b_vault: get_associated_token_address(&pool, &token_b),
            lp_token_mint: lp_mint,

            user_wallet: user.pubkey(),
            user_token_a_ata: get_associated_token_address(&user.pubkey(), &token_a),
            user_token_b_ata: get_associated_token_address(&user.pubkey(), &token_b),
            user_lp_ata: get_associated_token_address(&user.pubkey(), &lp_mint),

            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
            associated_token_program: spl_associated_token_account::id(),
        })
        .args(gfx_swap::instruction::Deposit2 {
            lp_token_amount: lpmint.decimals.apply(lp_amount),
            maximum_token_a_amount: amint.decimals.apply(1000000),
            maximum_token_b_amount: bmint.decimals.apply(1000000),
        })
        .signer(user)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "Deposit2: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );
}

#[throws(Error)]
pub fn swap_impl<N: AsPrimitive<f64>>(
    program: &Program,
    pool: Pubkey,
    lp_mint: Pubkey,
    swap_in: Pubkey,
    swap_out: Pubkey,
    user: &Keypair,
    amount: N,
) {
    let swap_in_mint: Mint = program.account(swap_in)?;
    let tx = program
        .request()
        .accounts(gfx_swap::accounts::Swap {
            pool: pool,

            in_token_vault: get_associated_token_address(&pool, &swap_in),
            out_token_vault: get_associated_token_address(&pool, &swap_out),
            lp_token_mint: lp_mint,
            fee_vault: get_associated_token_address(&pool, &lp_mint),

            user_wallet: user.pubkey(),
            in_token_ata_user: get_associated_token_address(&user.pubkey(), &swap_in),
            out_token_ata_user: get_associated_token_address(&user.pubkey(), &swap_out),

            token_program: spl_token::id(),
        })
        .args(gfx_swap::instruction::Swap {
            amount_in: swap_in_mint.decimals.apply(amount),
            minimum_amount_out: 0,
        })
        .signer(user)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!("Swap: https://explorer.solana.com/tx/{}?cluster=devnet", tx);
}

#[throws(Error)]
pub fn withdraw_fee_impl(program: &Program, pool: Pubkey, lp_mint: Pubkey, admin: &Keypair) {
    let tx = program
        .request()
        .accounts(gfx_swap::accounts::WithdrawFee {
            pool: pool,

            lp_token_mint: lp_mint,
            fee_vault: get_associated_token_address(&pool, &lp_mint),
            admin_ata: get_associated_token_address(&admin.pubkey(), &lp_mint),
            admin: admin.pubkey(),

            token_program: spl_token::id(),
        })
        .args(gfx_swap::instruction::WithdrawFee {})
        .signer(admin)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "WithdrawFee: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );
}
