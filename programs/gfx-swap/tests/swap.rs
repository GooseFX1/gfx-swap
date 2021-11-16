mod utils;

use anchor_client::{Client, Cluster};
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Error;
use fehler::throws;
use gfx_solana_utils::{admin_wallet, create_token, mint_to, user_wallet, ApplyDecimal, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, LPMint, Pool};
use serial_test::serial;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signature::Signer};
use spl_associated_token_account::get_associated_token_address;

#[throws(Error)]
#[serial]
#[test]
fn swap_and_withdraw_fee() {
    let _ = env_logger::try_init();

    let admin = admin_wallet(1.)?;
    let user = user_wallet(1.)?;
    println!("Admin: {}", admin.pubkey());
    println!("User: {}", user.pubkey());

    let client = Client::new_with_options(
        Cluster::Devnet,
        admin.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(gfx_swap::ID);

    let token_a = create_token(&admin)?;
    let token_b = create_token(&admin)?;
    let token_a_mint: Mint = program.account(token_a)?;
    let token_b_mint: Mint = program.account(token_b)?;

    println!("Token A mint: {}", token_a);
    println!("Token B mint: {}", token_b);

    mint_to(token_a, admin, user.pubkey(), 1000)?;
    mint_to(token_b, admin, user.pubkey(), 1000)?;

    // used for bootstrap
    mint_to(token_a, admin, admin.pubkey(), 1)?;
    mint_to(token_b, admin, admin.pubkey(), 1)?;

    // seed for creating the pool
    let seed = Keypair::new().pubkey();

    let (pool, pool_bump) = Pool::get_address_with_bump(&gfx_swap::ID, &seed.to_bytes());
    println!("Pool: {}", pool);
    let token_a_vault_address = get_associated_token_address(&pool, &token_a);
    let token_b_vault_address = get_associated_token_address(&pool, &token_a);
    println!("Token A vault: {}", token_a_vault_address);
    println!("Token B vault: {}", token_b_vault_address);

    let (lp_mint, lp_bump) = LPMint::get_address_with_bump(&gfx_swap::ID, &seed.to_bytes());
    println!("LP: {}", lp_mint);
    let lp_vault_address = get_associated_token_address(&pool, &lp_mint);

    utils::create_pool_impl(
        &program,
        pool,
        lp_mint,
        token_a,
        token_b,
        &admin,
        seed.to_bytes(),
        pool_bump,
        lp_bump,
        true,
    )?;

    let token_a_vault: TokenAccount = program.account(token_a_vault_address)?;
    assert_eq!(token_a_vault.amount, 1);

    let token_b_vault: TokenAccount = program.account(token_b_vault_address)?;
    assert_eq!(token_b_vault.amount, 1);

    let lp_mint_account: Mint = program.account(lp_mint)?;
    assert_eq!(lp_mint_account.supply, 1);

    utils::deposit2_impl(&program, pool, lp_mint, token_a, token_b, user, 10.)?;

    let token_a_vault: TokenAccount = program.account(token_a_vault_address)?;
    assert_eq!(token_a_vault.amount, token_a_mint.decimals.apply(100) + 1);

    let token_b_vault: TokenAccount = program.account(token_b_vault_address)?;
    assert_eq!(token_b_vault.amount, token_b_mint.decimals.apply(100) + 1);

    let lp_mint_account: Mint = program.account(lp_mint)?;
    assert_eq!(lp_mint_account.supply, token_b_mint.decimals.apply(100) + 1);

    utils::swap_impl(&program, pool, lp_mint, token_a, token_b, user, 13.)?;

    let lp_fee_ata: TokenAccount = program.account(lp_vault_address)?;
    assert!(lp_fee_ata.amount != 0);

    utils::withdraw_fee_impl(&program, pool, lp_mint, admin)?;

    let lp_fee_ata: TokenAccount = program.account(lp_vault_address)?;
    assert!(lp_fee_ata.amount == 0);
}
