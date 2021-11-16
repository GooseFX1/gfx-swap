//! Swap calculations

use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

/// Helper function for mapping to ErrorCode::CalculationFailure
pub fn map_zero_to_none(x: u128) -> Option<u128> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// The direction of a trade, since curves can be specialized to treat each
/// token differently (by adding offsets or weights)git pull
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TradeDirection {
    /// Input token A, output token B
    AtoB,
    /// Input token B, output token A
    BtoA,
}

/// The direction to round.  Used for pool token to trading token conversions to
/// avoid losing value on any deposit or withdrawal.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoundDirection {
    /// Floor the value, ie. 1.9 => 1, 1.1 => 1, 1.5 => 1
    Floor,
    /// Ceiling the value, ie. 1.9 => 2, 1.1 => 2, 1.5 => 2
    Ceiling,
}

impl TradeDirection {
    /// Given a trade direction, gives the opposite direction of the trade, so
    /// A to B becomes B to A, and vice versa
    pub fn opposite(&self) -> TradeDirection {
        match self {
            TradeDirection::AtoB => TradeDirection::BtoA,
            TradeDirection::BtoA => TradeDirection::AtoB,
        }
    }
}

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapWithoutFeesResult {
    /// Amount of source token swapped
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
}

/// Encodes results of depositing both sides at once
#[derive(Debug, PartialEq)]
pub struct TradingTokenResult {
    /// Amount of token 1
    pub token1_amount: u128,
    /// Amount of token 2
    pub token2_amount: u128,
}

/// Test helpers for curves
#[cfg(test)]
pub mod test {
    use super::super::CurveCalculator;
    use super::*;
    use proptest::prelude::*;
    use spl_math::{precise_number::PreciseNumber, uint::U256};

    /// The epsilon for most curves when performing the conversion test,
    /// comparing a one-sided deposit to a swap + deposit.
    pub const CONVERSION_BASIS_POINTS_GUARANTEE: u128 = 50;

    /// Test function to check that depositing token A is the same as swapping
    /// half for token B and depositing both.
    /// Since calculations use unsigned integers, there will be truncation at
    /// some point, meaning we can't have perfect equality.
    /// We guarantee that the relative error between depositing one side and
    /// performing a swap plus deposit will be at most some epsilon provided by
    /// the curve. Most curves guarantee accuracy within 0.5%.
    pub fn check_deposit_token_conversion(
        curve: &dyn CurveCalculator,
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
        pool_supply: u128,
        epsilon_in_basis_points: u128,
    ) {
        let amount_to_swap = source_token_amount / 2;
        let results = curve
            .swap_without_fees(
                amount_to_swap,
                swap_source_amount,
                swap_destination_amount,
                trade_direction,
            )
            .unwrap();
        let opposite_direction = trade_direction.opposite();
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (swap_source_amount, swap_destination_amount),
            TradeDirection::BtoA => (swap_destination_amount, swap_source_amount),
        };

        // base amount
        let pool_tokens_from_one_side = curve
            .deposit_single_token_type(
                source_token_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                trade_direction,
            )
            .unwrap();

        // perform both separately, updating amounts accordingly
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (
                swap_source_amount + results.source_amount_swapped,
                swap_destination_amount - results.destination_amount_swapped,
            ),
            TradeDirection::BtoA => (
                swap_destination_amount - results.destination_amount_swapped,
                swap_source_amount + results.source_amount_swapped,
            ),
        };
        let pool_tokens_from_source = curve
            .deposit_single_token_type(
                source_token_amount - results.source_amount_swapped,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                trade_direction,
            )
            .unwrap();
        let pool_tokens_from_destination = curve
            .deposit_single_token_type(
                results.destination_amount_swapped,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply + pool_tokens_from_source,
                opposite_direction,
            )
            .unwrap();

        let pool_tokens_total_separate = pool_tokens_from_source + pool_tokens_from_destination;

        // slippage due to rounding or truncation errors
        let epsilon = std::cmp::max(
            1,
            pool_tokens_total_separate * epsilon_in_basis_points / 10000,
        );
        let difference = if pool_tokens_from_one_side >= pool_tokens_total_separate {
            pool_tokens_from_one_side - pool_tokens_total_separate
        } else {
            pool_tokens_total_separate - pool_tokens_from_one_side
        };
        assert!(
            difference <= epsilon,
            "difference expected to be less than {}, actually {}",
            epsilon,
            difference
        );
    }

    /// Test function to check that withdrawing token A is the same as withdrawing
    /// both and swapping one side.
    /// Since calculations use unsigned integers, there will be truncation at
    /// some point, meaning we can't have perfect equality.
    /// We guarantee that the relative error between withdrawing one side and
    /// performing a withdraw plus a swap will be at most some epsilon provided by
    /// the curve. Most curves guarantee accuracy within 0.5%.
    pub fn check_withdraw_token_conversion(
        curve: &dyn CurveCalculator,
        pool_token_amount: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        trade_direction: TradeDirection,
        epsilon_in_basis_points: u128,
    ) {
        // withdraw the pool tokens
        let withdraw_result = curve
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_token_supply,
                swap_token_a_amount,
                swap_token_b_amount,
                RoundDirection::Floor,
            )
            .unwrap();

        let new_swap_token_a_amount = swap_token_a_amount - withdraw_result.token1_amount;
        let new_swap_token_b_amount = swap_token_b_amount - withdraw_result.token2_amount;

        // swap one side of them
        let source_token_amount = match trade_direction {
            TradeDirection::AtoB => {
                let results = curve
                    .swap_without_fees(
                        withdraw_result.token1_amount,
                        new_swap_token_a_amount,
                        new_swap_token_b_amount,
                        trade_direction,
                    )
                    .unwrap();
                withdraw_result.token2_amount + results.destination_amount_swapped
            }
            TradeDirection::BtoA => {
                let results = curve
                    .swap_without_fees(
                        withdraw_result.token2_amount,
                        new_swap_token_b_amount,
                        new_swap_token_a_amount,
                        trade_direction,
                    )
                    .unwrap();
                withdraw_result.token1_amount + results.destination_amount_swapped
            }
        };

        // see how many pool tokens it would cost to withdraw one side for the
        // total amount of tokens, should be close!
        let opposite_direction = trade_direction.opposite();
        let pool_token_amount_from_single_side_withdraw = curve
            .withdraw_single_token_type_exact_out(
                source_token_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_token_supply,
                opposite_direction,
            )
            .unwrap();

        // slippage due to rounding or truncation errors
        let epsilon = std::cmp::max(1, pool_token_amount * epsilon_in_basis_points / 10000);
        let difference = if pool_token_amount >= pool_token_amount_from_single_side_withdraw {
            pool_token_amount - pool_token_amount_from_single_side_withdraw
        } else {
            pool_token_amount_from_single_side_withdraw - pool_token_amount
        };
        assert!(
            difference <= epsilon,
            "difference expected to be less than {}, actually {}",
            epsilon,
            difference
        );
    }

    /// Test function checking that a swap never reduces the overall value of
    /// the pool.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost in
    /// either direction if too much is given to the swapper.
    ///
    /// This test guarantees that the relative change in value will be at most
    /// 1 normalized token, and that the value will never decrease from a trade.
    pub fn check_curve_value_from_swap(
        curve: &dyn CurveCalculator,
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) {
        let results = curve
            .swap_without_fees(
                source_token_amount,
                swap_source_amount,
                swap_destination_amount,
                trade_direction,
            )
            .unwrap();

        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (swap_source_amount, swap_destination_amount),
            TradeDirection::BtoA => (swap_destination_amount, swap_source_amount),
        };
        let previous_value = curve
            .normalized_value(swap_token_a_amount, swap_token_b_amount)
            .unwrap();

        let new_swap_source_amount = swap_source_amount
            .checked_add(results.source_amount_swapped)
            .unwrap();
        let new_swap_destination_amount = swap_destination_amount
            .checked_sub(results.destination_amount_swapped)
            .unwrap();
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (new_swap_source_amount, new_swap_destination_amount),
            TradeDirection::BtoA => (new_swap_destination_amount, new_swap_source_amount),
        };

        let new_value = curve
            .normalized_value(swap_token_a_amount, swap_token_b_amount)
            .unwrap();
        assert!(new_value.greater_than_or_equal(&previous_value));

        let epsilon = 1; // Extremely close!
        let difference = new_value
            .checked_sub(&previous_value)
            .unwrap()
            .to_imprecise()
            .unwrap();
        assert!(difference <= epsilon);
    }

    /// Test function checking that a deposit never reduces the value of pool
    /// tokens.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost if
    /// too much is given to the depositor.
    pub fn check_pool_value_from_deposit(
        curve: &dyn CurveCalculator,
        pool_token_amount: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) {
        let deposit_result = curve
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_token_supply,
                swap_token_a_amount,
                swap_token_b_amount,
                RoundDirection::Ceiling,
            )
            .unwrap();
        let new_swap_token_a_amount = swap_token_a_amount + deposit_result.token1_amount;
        let new_swap_token_b_amount = swap_token_b_amount + deposit_result.token2_amount;
        let new_pool_token_supply = pool_token_supply + pool_token_amount;

        // the following inequality must hold:
        // new_token_a / new_pool_token_supply >= token_a / pool_token_supply
        // which reduces to:
        // new_token_a * pool_token_supply >= token_a * new_pool_token_supply

        // These numbers can be just slightly above u64 after the deposit, which
        // means that their multiplication can be just above the range of u128.
        // For ease of testing, we bump these up to U256.
        let pool_token_supply = U256::from(pool_token_supply);
        let new_pool_token_supply = U256::from(new_pool_token_supply);
        let swap_token_a_amount = U256::from(swap_token_a_amount);
        let new_swap_token_a_amount = U256::from(new_swap_token_a_amount);
        let swap_token_b_amount = U256::from(swap_token_b_amount);
        let new_swap_token_b_amount = U256::from(new_swap_token_b_amount);

        assert!(
            new_swap_token_a_amount * pool_token_supply
                >= swap_token_a_amount * new_pool_token_supply
        );
        assert!(
            new_swap_token_b_amount * pool_token_supply
                >= swap_token_b_amount * new_pool_token_supply
        );
    }

    /// Test function checking that a withdraw never reduces the value of pool
    /// tokens.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost if
    /// too much is given to the depositor.
    pub fn check_pool_value_from_withdraw(
        curve: &dyn CurveCalculator,
        pool_token_amount: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) {
        let withdraw_result = curve
            .pool_tokens_to_trading_tokens(
                pool_token_amount,
                pool_token_supply,
                swap_token_a_amount,
                swap_token_b_amount,
                RoundDirection::Floor,
            )
            .unwrap();
        let new_swap_token_a_amount = swap_token_a_amount - withdraw_result.token1_amount;
        let new_swap_token_b_amount = swap_token_b_amount - withdraw_result.token2_amount;
        let new_pool_token_supply = pool_token_supply - pool_token_amount;

        let value = curve
            .normalized_value(swap_token_a_amount, swap_token_b_amount)
            .unwrap();
        // since we can get rounding issues on the pool value which make it seem that the
        // value per token has gone down, we bump it up by an epsilon of 1 to
        // cover all cases
        let new_value = curve
            .normalized_value(new_swap_token_a_amount, new_swap_token_b_amount)
            .unwrap();

        // the following inequality must hold:
        // new_pool_value / new_pool_token_supply >= pool_value / pool_token_supply
        // which can also be written:
        // new_pool_value * pool_token_supply >= pool_value * new_pool_token_supply

        let pool_token_supply = PreciseNumber::new(pool_token_supply).unwrap();
        let new_pool_token_supply = PreciseNumber::new(new_pool_token_supply).unwrap();
        assert!(new_value
            .checked_mul(&pool_token_supply)
            .unwrap()
            .greater_than_or_equal(&value.checked_mul(&new_pool_token_supply).unwrap()));
    }

    prop_compose! {
        pub fn total_and_intermediate()(total in 1..u64::MAX)
                        (intermediate in 1..total, total in Just(total))
                        -> (u64, u64) {
           (total, intermediate)
       }
    }
}