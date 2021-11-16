mod utils;

use anchor_client::{Client, Cluster};
use anyhow::Error;
use fehler::throws;
use gfx_solana_utils::{admin_wallet, create_token, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, LPMint, Pool};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signature::Signer};

#[throws(Error)]
#[serial]
#[test]
fn create_swap_pool() {
    let _ = env_logger::try_init();

    let admin = admin_wallet(1.)?;

    let client = Client::new(Cluster::Devnet, admin.clone());
    let program = client.program(gfx_swap::ID);

    let token_a = create_token(&admin)?;

    let token_b = create_token(&admin)?;

    // seed for creating the pool
    let seed = Keypair::new().pubkey();

    let (pool, pool_bump) = Pool::get_address_with_bump(&gfx_swap::ID, &seed.to_bytes());
    let (lp_mint, lp_bump) = LPMint::get_address_with_bump(&gfx_swap::ID, &seed.to_bytes());

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
        false,
    )?;
}
