use anchor_client::{Client, Cluster};
use anchor_spl::token::Mint;
use anyhow::Result;
use gfx_solana_utils::{load_keypair, AnchorClientErrorExt, ApplyDecimal, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, ErrorCode, LPMint, Pool};
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
    user_wallet: String,

    #[structopt(long, env)]
    token_a: Pubkey,

    #[structopt(long, env)]
    token_b: Pubkey,

    #[structopt(long)]
    swap_in: String,

    #[structopt(long)]
    amount: f64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let program_id = gfx_swap::ID;
    println!("program_id: {}", program_id);

    let user = load_keypair(&opt.user_wallet)?;

    let lp_mint = LPMint::get_address(&program_id, &opt.seed.to_bytes());
    let pool = Pool::get_address(&program_id, &opt.seed.to_bytes());

    let client =
        Client::new_with_options(Cluster::Devnet, user.clone(), CommitmentConfig::confirmed());
    let program = client.program(program_id);
    let (swap_in, swap_out, swap_in_mint): (_, _, Mint) = match opt.swap_in.as_str() {
        "A" => (opt.token_a, opt.token_b, program.account(opt.token_a)?),
        "B" => (opt.token_b, opt.token_a, program.account(opt.token_b)?),
        _ => {
            panic!("--swap-in can only by A or B")
        }
    };

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
            amount_in: swap_in_mint.decimals.apply(opt.amount),
            minimum_amount_out: 0,
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
