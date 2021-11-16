use super::DerivedAccountIdentifier;

impl DerivedAccountIdentifier for LPMint {
    const IDENT: &'static [u8] = b"GFXLPMint";
}

// We do not actually instantiate this state. Just use this one for verifying the address
pub struct LPMint {
    _unused: u8,
}
