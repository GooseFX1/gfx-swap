use crate::states::Pool;
use anchor_lang::prelude::*;
use fehler::throws;

#[throws(ProgramError)]
pub fn suspended(pool: &Account<'_, Pool>) {
    require!(!pool.suspended, Suspended);
}
