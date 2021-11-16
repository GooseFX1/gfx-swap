mod lp_mint;
mod pool;

pub use lp_mint::LPMint;
pub use pool::{Pool, PoolConfig, PoolExt};

use crate::errors::ErrorCode::*;
use anchor_lang::prelude::*;
use fehler::{throw, throws};

// All the PDA account of this program are derived from a same seed with `find_program_address(IDENT, seed)`.
pub trait DerivedAccountIdentifier {
    const IDENT: &'static [u8];

    fn get_address(program_id: &Pubkey, seed: &[u8]) -> Pubkey {
        Self::get_address_with_bump(program_id, seed).0
    }

    fn get_address_with_bump(program_id: &Pubkey, seed: &[u8]) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[Self::IDENT, seed], program_id)
    }

    #[throws(ProgramError)]
    fn verify_address(program_id: &Pubkey, seed: &[u8], address: &Pubkey) {
        let (expected, _) = Self::get_address_with_bump(program_id, seed);

        if &expected != address {
            throw!(ContractAddressNotCorrect);
        }
    }

    #[throws(ProgramError)]
    fn verify_address_with_bump(program_id: &Pubkey, seed: &[u8], bump: u8, address: &Pubkey) {
        let addr = Pubkey::create_program_address(&[Self::IDENT, seed, &[bump]], program_id)?;

        if &addr != address {
            throw!(ContractAddressNotCorrect);
        }
    }
}
