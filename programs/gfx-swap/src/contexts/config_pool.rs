use crate::states::{DerivedAccountIdentifier, Pool, PoolConfig};
use crate::ErrorCode::*;
use anchor_lang::prelude::*;
use fehler::throws;

#[derive(Accounts)]
pub struct ConfigPool<'info> {
    #[account(
        seeds = [Pool::IDENT, &pool.seed],
        bump = pool.bump,
        has_one = admin @ WrongAdmin
    )]
    pub pool: Account<'info, Pool>,

    pub admin: Signer<'info>, // admin account can do privileged operations
}

impl<'info> ConfigPool<'info> {
    #[throws(ProgramError)]
    pub fn process(&mut self, config: &PoolConfig) {
        let ConfigPool { pool, .. } = self;
        pool.config(&config);
    }
}
