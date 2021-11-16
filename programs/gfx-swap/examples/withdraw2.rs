use anchor_client::{Client, Cluster};
use anchor_spl::token::Mint;
use anyhow::Result;
use gfx_solana_utils::{load_keypair, AnchorClientErrorExt, ApplyDecimal, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, ErrorCode, LPMint, Pool, LP_TOKEN_DECIMALS};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer};
use spl_associated_token_account::get_associated_token_address;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "transact", about = "Making transactions to the GFX Swap")]
struct Opt {
    #[structopt(long, env, short = "p")]
    program_id: Option<Pubkey>,

    #[structopt(long, env)]
    seed: Pubkey,

    #[structopt(long, env)]
    token_a: Pubkey,

    #[structopt(long, env)]
    token_b: Pubkey,

    #[structopt(long, env)]
    user_wallet: String,

    #[structopt(long, env)]
    lp_amount: f64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let program_id = gfx_swap::ID;
    println!("program_id: {}", program_id);

    let user = load_keypair(&opt.user_wallet)?;

    let pool = Pool::get_address(&program_id, &opt.seed.to_bytes());
    let lp_mint = LPMint::get_address(&program_id, &opt.seed.to_bytes());

    let client =
        Client::new_with_options(Cluster::Devnet, user.clone(), CommitmentConfig::confirmed());
    let program = client.program(program_id);

    let amint: Mint = program.account(opt.token_a)?;
    let bmint: Mint = program.account(opt.token_b)?;

    let tx = program
        .request()
        .accounts(gfx_swap::accounts::Withdraw2 {
            pool: pool,
            token_a_vault: get_associated_token_address(&pool, &opt.token_a),
            token_b_vault: get_associated_token_address(&pool, &opt.token_b),
            lp_token_mint: lp_mint,
            fee_vault: get_associated_token_address(&pool, &lp_mint),

            user_wallet: user.pubkey(),
            user_token_a_ata: get_associated_token_address(&user.pubkey(), &opt.token_a),
            user_token_b_ata: get_associated_token_address(&user.pubkey(), &opt.token_b),
            user_lp_ata: get_associated_token_address(&user.pubkey(), &lp_mint),

            token_program: spl_token::id(),
        })
        .args(gfx_swap::instruction::Withdraw2 {
            lp_token_amount: LP_TOKEN_DECIMALS.apply(opt.lp_amount),
            minimum_token_a_amount: amint.decimals.apply(0),
            minimum_token_b_amount: bmint.decimals.apply(0),
        })
        .signer(&user)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "Transaction: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );

    Ok(())
}
