//! Base curve implementation

use super::{
    calculators::{
        ConstantProductCurve, CurveCalculator, RoundDirection, StableCurve, SwapWithoutFeesResult,
        TradeDirection, TradingTokenResult,
    },
    fees::Fees,
};
use crate::ErrorCode;
use anchor_lang::prelude::*;
use enum_dispatch::enum_dispatch;
use spl_math::precise_number::PreciseNumber;
use std::fmt::Debug;

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapResult {
    /// New amount of source token
    pub new_swap_source_amount: u128,
    /// New amount of destination token
    pub new_swap_destination_amount: u128,
    /// Amount of source token swapped (includes fees)
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
    /// Amount of source tokens going to pool holders
    pub trade_fee: u128,
    /// Amount of source tokens going to owner
    pub owner_fee: u128,
}

/// All the supported curve types. We do not use the trait object solution in the SPL.
/// Instead, we use enums.
#[enum_dispatch]
#[repr(C)]
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub enum SwapCurve {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProductCurve,
    /// Stable, like uniswap, but with wide zone of 1:1 instead of one point
    StableCurve,
}

impl Default for SwapCurve {
    fn default() -> Self {
        Self::ConstantProductCurve(ConstantProductCurve::default())
    }
}

impl SwapCurve {
    /// Subtract fees and calculate how much destination token will be provided
    /// given an amount of source token.
    pub fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = fees.trading_fee(source_amount)?;
        let owner_fee = fees.owner_trading_fee(source_amount)?;

        let total_fees = trade_fee.checked_add(owner_fee)?;
        let source_amount_less_fees = source_amount.checked_sub(total_fees)?;

        let SwapWithoutFeesResult {
            source_amount_swapped,
            destination_amount_swapped,
        } = self.swap_without_fees(
            source_amount_less_fees,
            swap_source_amount,
            swap_destination_amount,
            trade_direction,
        )?;

        let source_amount_swapped = source_amount_swapped.checked_add(total_fees)?;
        Some(SwapResult {
            new_swap_source_amount: swap_source_amount.checked_add(source_amount_swapped)?,
            new_swap_destination_amount: swap_destination_amount
                .checked_sub(destination_amount_swapped)?,
            source_amount_swapped,
            destination_amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Get the amount of pool tokens for the deposited amount of token A or B
    pub fn deposit_single_token_type(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let source_amount = source_amount.checked_sub(trade_fee)?;

        CurveCalculator::deposit_single_token_type(
            self,
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
        )
    }

    /// Get the amount of pool tokens for the withdrawn amount of token A or B
    pub fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let source_amount = source_amount.checked_sub(trade_fee)?;

        CurveCalculator::deposit_single_token_type(
            self,
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_product_trade_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let source_amount = 100;
        let swap_curve = SwapCurve::ConstantProductCurve(ConstantProductCurve::new());
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4504);
        assert_eq!(result.new_swap_destination_amount, 45496);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 0);
    }

    #[test]
    fn constant_product_owner_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 0;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let source_amount: u128 = 100;
        let swap_curve = SwapCurve::ConstantProductCurve(ConstantProductCurve::default());
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4504);
        assert_eq!(result.new_swap_destination_amount, 45496);
        assert_eq!(result.trade_fee, 0);
        assert_eq!(result.owner_fee, 1);
    }

    #[test]
    fn constant_product_no_fee() {
        let swap_source_amount: u128 = 1_000;
        let swap_destination_amount: u128 = 50_000;
        let source_amount: u128 = 100;
        let fees = Fees::default();
        let swap_curve = SwapCurve::ConstantProductCurve(ConstantProductCurve::default());
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4545);
        assert_eq!(result.new_swap_destination_amount, 45455);
    }
}
