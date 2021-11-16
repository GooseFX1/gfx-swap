use anchor_client::{Client, Cluster};
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use gfx_solana_utils::{load_keypair, ApplyDecimal, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, LPMint, Pool};
use prettytable::{cell, row, table};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "transact", about = "Making transactions to the GFX SWap")]
struct Opt {
    #[structopt(long, env, short = "p")]
    program_id: Option<Pubkey>,

    #[structopt(long, env)]
    seed: Pubkey,

    #[structopt(long, env)]
    admin_wallet: String,

    #[structopt(long, env)]
    token_a: Pubkey,

    #[structopt(long, env)]
    token_b: Pubkey,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let program_id = gfx_swap::ID;
    println!("program_id: {}", program_id);

    let admin = load_keypair(&opt.admin_wallet)?;

    let client = Client::new_with_options(
        Cluster::Devnet,
        admin.clone(),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(program_id);

    let pool = Pool::get_address(&program_id, &opt.seed.to_bytes());

    let lp_mint = LPMint::get_address(&program_id, &opt.seed.to_bytes());

    let token_a_mint: Mint = program.account(opt.token_a)?;
    let token_a_ata: TokenAccount =
        program.account(get_associated_token_address(&pool, &opt.token_a))?;
    let token_b_mint: Mint = program.account(opt.token_b)?;
    let token_b_ata: TokenAccount =
        program.account(get_associated_token_address(&pool, &opt.token_b))?;

    let lp_mint_account: Mint = program.account(lp_mint)?;
    let lp_fee_ata: TokenAccount =
        program.account(get_associated_token_address(&pool, &lp_mint))?;

    let table = table!(
        ["Name", "Amount", "Token Address"],
        [
            "Token A in Pool",
            token_a_mint.decimals.unapply(token_a_ata.amount),
            opt.token_a
        ],
        [
            "Token B in Pool",
            token_b_mint.decimals.unapply(token_b_ata.amount),
            opt.token_b
        ],
        [
            "Owner Fee",
            lp_mint_account.decimals.unapply(lp_fee_ata.amount),
            lp_mint
        ],
        [
            "Total LP",
            lp_mint_account.decimals.unapply(lp_mint_account.supply),
            lp_mint
        ]
    );

    table.printstd();

    Ok(())
}
