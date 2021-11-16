use anchor_lang::prelude::*;
use std::convert::TryInto;

// Define errors, custom error code: 300 + idx => 0x12C + 0x${idx}
#[error(offset = 300)]
pub enum ErrorCode {
    #[msg("[G000] Contract address is not correct")] //0x12C (300)
    ContractAddressNotCorrect,

    #[msg("[G001]  Input token account empty")] //0x12D (301)
    EmptySupply,

    #[msg("[G002] The provided fee does not match the program owner's constraints")] //0x12E (302)
    InvalidFee,

    #[msg("[G003] The provided curve parameters are invalid")] //0x12F (303)
    InvalidCurve,

    #[msg("[G004] Given pool token amount results in zero trading tokens")] //0x130 (304)
    ZeroTradingTokens,

    #[msg("[G005] Swap instruction exceeds desired slippage limit")] //0x131 (305)
    ExceededSlippage,

    #[msg("[G006] Conversion to u64 failed with an overflow or underflow")] //0x132 (306)
    ConversionFailure,

    #[msg("[G007] The operation cannot be performed on the given curve")] //0x132 (307)
    UnsupportedCurveOperation,

    #[msg("[G008] Fee calculation failed due to overflow, underflow, or unexpected 0")]
    //0x133 (308)
    FeeCalculationFailure,

    #[msg("[G009] Privilege required to execute this method")] //0x134 (309)
    PrivilegeRequired,

    #[msg("[G010] The Mint for the LP token is not correct")] //0x135 (310)
    WrongLPMint,

    #[msg("[G011] The tokens are same")] //0x136 (311)
    SameToken,

    #[msg("[G012] General calculation failure due to overflow or underflow")] //0x137 (312)
    CalculationFailure,

    #[msg("[G013] Address of the provided swap token account is incorrect")] //0x138 (313)
    IncorrectSwapAccount,

    #[msg("[G014] Two of the tokens have different Mint")] //0x139 (314)
    MintMismatch,

    #[msg("[G015] The mint of the token is not expected")] //0x13A (315)
    MintNotExpected,

    #[msg("[G016] Wrong owner of the associated token account")] //0x13B (316)
    WrongATAOwner,

    #[msg("[G017] Pool does not support the token")] //0x13C (317)
    TokenNotSupportedByPool,

    #[msg("[G018] Wrong admin")] //0x13D (318)
    WrongAdmin,

    #[msg("[G019] Pool is suspended")] //0x13E (319)
    Suspended,

    #[msg("[G020] Wrong fee vault")] //0x13F (320)
    WrongFeeVault,
}

impl TryInto<ErrorCode> for u32 {
    type Error = (); // Error if u32 is out of range

    fn try_into(self) -> std::result::Result<ErrorCode, ()> {
        if (300..=320).contains(&self) {
            Ok(unsafe { std::mem::transmute(self - 300) })
        } else {
            Err(())
        }
    }
}
