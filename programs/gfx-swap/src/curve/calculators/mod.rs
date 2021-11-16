mod constant_product;
mod stable;
mod types;

pub use constant_product::ConstantProductCurve;
pub use stable::StableCurve;
pub use types::{
    map_zero_to_none, RoundDirection, SwapWithoutFeesResult, TradeDirection, TradingTokenResult,
    INITIAL_SWAP_POOL_AMOUNT,
};

use crate::errors::ErrorCode::{self, *};
use enum_dispatch::enum_dispatch;
use spl_math::precise_number::PreciseNumber;

/// Trait representing operations required on a swap curve
#[enum_dispatch(SwapCurve)]
pub trait CurveCalculator {
    /// Calculate how much destination token will be provided given an amount
    /// of source token.
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult>;

    /// Get the supply for a new pool
    /// The default implementation is a Balancer-style fixed initial supply
    fn new_pool_supply(&self) -> u128 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        swap_token1_amount: u128,
        swap_token2_amount: u128,
        round_direction: RoundDirection,
    ) -> Option<TradingTokenResult>;

    /// Get the amount of pool tokens for the deposited amount of token A or B.
    ///
    /// This is used for single-sided deposits.  It essentially performs a swap
    /// followed by a deposit.  Because a swap is implicitly performed, this will
    /// change the spot price of the pool.
    ///
    /// See more background for the calculation at:
    ///
    /// <https://balancer.finance/whitepaper/#single-asset-deposit-withdrawal>
    fn deposit_single_token_type(
        &self,
        source_amount: u128,
        swap_token1_amount: u128,
        swap_token2_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
    ) -> Option<u128>;

    /// Get the amount of pool tokens for the withdrawn amount of token A or B.
    ///
    /// This is used for single-sided withdrawals and owner trade fee
    /// calculation. It essentially performs a withdrawal followed by a swap.
    /// Because a swap is implicitly performed, this will change the spot price
    /// of the pool.
    ///
    /// See more background for the calculation at:
    ///
    /// <https://balancer.finance/whitepaper/#single-asset-deposit-withdrawal>
    fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token1_amount: u128,
        swap_token2_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
    ) -> Option<u128>;

    /// Validate that the given curve has no invalid parameters
    fn validate(&self) -> Result<(), ErrorCode>;

    /// Validate the given supply on initialization. This is useful for curves
    /// that allow zero supply on one or both sides, since the standard constant
    /// product curve must have a non-zero supply on both sides.
    fn validate_supply(&self, token1_amount: u64, token2_amount: u64) -> Result<(), ErrorCode> {
        if token1_amount == 0 {
            return Err(EmptySupply);
        }
        if token2_amount == 0 {
            return Err(EmptySupply);
        }
        Ok(())
    }

    /// Some curves function best and prevent attacks if we prevent deposits
    /// after initialization.  For example, the offset curve in `offset.rs`,
    /// which fakes supply on one side of the swap, allows the swap creator
    /// to steal value from all other depositors.
    fn allows_deposits(&self) -> bool {
        true
    }

    /// Calculates the total normalized value of the curve given the liquidity
    /// parameters.
    ///
    /// This value must have the dimension of `tokens ^ 1` For example, the
    /// standard Uniswap invariant has dimension `tokens ^ 2` since we are
    /// multiplying two token values together.  In order to normalize it, we
    /// also need to take the square root.
    ///
    /// This is useful for testing the curves, to make sure that value is not
    /// lost on any trade.  It can also be used to find out the relative value
    /// of pool tokens or liquidity tokens.
    fn normalized_value(
        &self,
        swap_token1_amount: u128,
        swap_token2_amount: u128,
    ) -> Option<PreciseNumber>;
}
