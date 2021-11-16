use anchor_client::{Client, Cluster};
use anyhow::Result;
use gfx_solana_utils::{load_keypair, AnchorClientErrorExt, Duplicate};
use gfx_swap::{
    ConstantProductCurve, DerivedAccountIdentifier, ErrorCode, Fees, LPMint, Pool, SwapCurve,
};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer, system_program, sysvar,
    transaction::Transaction,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::instruction::transfer;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "transact", about = "Making transactions to the GFX Swap")]
struct Opt {
    #[structopt(long, env, short = "p")]
    program_id: Option<Pubkey>,

    #[structopt(long, env)]
    admin_wallet: String,

    #[structopt(long, env)]
    user_wallet: String,

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
    let user_wallet = load_keypair(&opt.user_wallet)?;

    let client = Client::new_with_options(
        Cluster::Devnet,
        admin.clone(),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(program_id);

    // seed for creating the pool
    let seed = solana_sdk::signature::Keypair::new().pubkey();

    let (pool, pool_bump) = Pool::get_address_with_bump(&program_id, &seed.to_bytes());
    let (lp_mint, lp_bump) = LPMint::get_address_with_bump(&program_id, &seed.to_bytes());

    println!("Creating the LP pool ...");
    let tx = program
        .request()
        .accounts(gfx_swap::accounts::CreatePool {
            pool: pool,

            lp_token_mint: lp_mint,
            fee_vault: get_associated_token_address(&pool, &lp_mint),

            token_a_mint: opt.token_a,
            token_a_vault: get_associated_token_address(&pool, &opt.token_a),

            token_b_mint: opt.token_b,
            token_b_vault: get_associated_token_address(&pool, &opt.token_b),

            admin: admin.pubkey(),

            associated_token_program: spl_associated_token_account::id(),
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
        })
        .args(gfx_swap::instruction::CreatePool {
            seed: seed.to_bytes(),
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
        .signer(&admin)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!(
        "Transaction: https://explorer.solana.com/tx/{}?cluster=devnet",
        tx
    );

    println!("Creating ATA account for the user ...");
    let rpc_client = program.rpc();

    println!("[Bootstrap] transfering LP token to the user ...");
    // transfer some lp token to the user
    program
        .request()
        .accounts(gfx_swap::accounts::MintLPTo {
            admin: user_wallet.pubkey(),
            pool,
            recipient_ata: get_associated_token_address(&user_wallet.pubkey(), &lp_mint),
            lp_token_mint: lp_mint,
            token_program: spl_token::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
            associated_token_program: spl_associated_token_account::id(),
        })
        .args(gfx_swap::instruction::MintLpTo { n: 1 })
        .instruction(
            // create the ata account of the lp token for the user
            create_associated_token_account(&admin.pubkey(), &user_wallet.pubkey(), &lp_mint),
        )
        .signer(&user_wallet)
        .send()
        .map_err(|e| e.canonicalize::<ErrorCode>())?;

    println!("[Bootstrap] transfering token A & B to the pool ...");

    // transfer some token a & b to the pool
    rpc_client.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
        &[
            transfer(
                &spl_token::id(),
                &get_associated_token_address(&user_wallet.pubkey(), &opt.token_a),
                &get_associated_token_address(&pool, &opt.token_a),
                &user_wallet.pubkey(),
                &[],
                1,
            )?,
            transfer(
                &spl_token::id(),
                &get_associated_token_address(&user_wallet.pubkey(), &opt.token_b),
                &get_associated_token_address(&pool, &opt.token_b),
                &user_wallet.pubkey(),
                &[],
                1,
            )?,
        ],
        Some(&admin.pubkey()),
        &[&admin, &user_wallet],
        rpc_client.get_recent_blockhash()?.0,
    ))?;

    println!(
        "Set the seed in your environment variables\n============\nSEED={}",
        seed
    );
    Ok(())
}
