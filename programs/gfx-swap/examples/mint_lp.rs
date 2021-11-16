use anchor_client::{Client, Cluster};
use anyhow::Result;
use gfx_solana_utils::{load_keypair, AnchorClientErrorExt, Duplicate};
use gfx_swap::{DerivedAccountIdentifier, ErrorCode, LPMint, Pool};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer, system_program, sysvar,
};
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
    mint_to: String,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let program_id = gfx_swap::ID;
    println!("program_id: {}", program_id);

    let mint_to = load_keypair(&opt.mint_to)?;

    let client = Client::new_with_options(
        Cluster::Devnet,
        mint_to.clone(),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(program_id);

    let pool = Pool::get_address(&program_id, &opt.seed.to_bytes());
    let lp_mint = LPMint::get_address(&program_id, &opt.seed.to_bytes());

    let tx = program
        .request()
        .accounts(gfx_swap::accounts::MintLPTo {
            admin: mint_to.pubkey(),
            pool,
            recipient_ata: get_associated_token_address(&mint_to.pubkey(), &lp_mint),
            lp_token_mint: lp_mint,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
            associated_token_program: spl_associated_token_account::id(),
        })
        .args(gfx_swap::instruction::MintLpTo { n: 1 })
        .signer(&mint_to)
        .options(CommitmentConfig::confirmed())
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "Transaction: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );

    Ok(())
}
