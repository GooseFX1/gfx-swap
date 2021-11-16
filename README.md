# GFX Swap Program

This is a re-implement of the swap program in solana-program-library https://github.com/solana-labs/solana-program-library/tree/master/token-swap.

## Audit

This repo is audited by Halborn. The auditing report is under [audit/GooseFX_Swap_Program_Security_Audit_Report_Halborn_Final.pdf](audit/GooseFX_Swap_Program_Security_Audit_Report_Halborn_Final.pdf)

## Deploy and run gfx-swap

### Prerequisites

* System: Mac or Linux
* [Rust](https://rustup.rs/)
* [Just](https://github.com/casey/just#pre-built-binaries)
* [Solana](https://docs.solana.com/cli/install-solana-cli-tools#macos--linux)
* [Anchor](https://project-serum.github.io/anchor/getting-started/installation.html#install-anchor)

### Set the required environment variables to a `.env` file
```
RUST_BACKTRACE=1
RUST_LOG=gfx_swap=info

TOKEN_A=2uig6CL6aQNS8wPL9YmfRNUNcQMgq9purmXK53pzMaQ6
TOKEN_B=8FUPzLY58ojDaj5yh1MKwyJnGNhCDMbStbHNVkBQ9KjJ

ADMIN_WALLET="~/.config/solana/id.json"
USER_WALLET="~/.config/solana/id.json"
```

You might want to provide different addresses for TOKEN_A and TOKEN_B.

### Generate a key for deploy the program
Run `just keygen gfx-swap`.

### Deploy the program

Run `just deploy gfx-swap --features ci`.

### Create a swap pool

Run `just cli create_pool`.

`create_pool` will create the associated token account (ATA) for `USER_WALLET`.

This command will print out a bunch of information, in which you need to write down the seed into .env. 
The seed is a unique identifier to the pool.

### Pool monitor

Run `just cli pool_status`. 
This will print out a snapshot of the current pool status.
You can combine it with the `watch` command, i.e. `watch just cli pool_status` to constantly monitor the pool.

### Deposit tokens into the pool

Run `just cli deposit2 --lp-amount <amount>`. 
This basically askes the pool `I'd like to get <amount> LP tokens and you can deduct some amount of token A and B from my wallet for that.`

### Withdraw tokens from the pool

Run `just cli withdraw2 --lp-amount <amount>`. 
This is the inverse operation to the `deposit` command.

### Swap tokens

Run `just cli swap --swap-in A --amount <amount>`. 
This swaps in some token A for some token B.

