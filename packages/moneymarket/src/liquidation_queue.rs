use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Uint128, HumanAddr};
use cw20::Cw20ReceiveMsg;

use crate::tokens::TokensHuman;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: HumanAddr,
    pub oracle_contract: HumanAddr,
    pub stable_addr: HumanAddr,
    /// borrow_amount / borrow_limit must always be bigger than  
    /// safe_ratio.
    pub safe_ratio: Decimal256,
    /// Fee applied to executed bids
    /// Sent to Overseer interest buffer
    pub bid_fee: Decimal256,
    /// Fee applied to executed bids
    /// Sent to the address executing the liquidation
    pub liquidator_fee: Decimal256,
    /// Liquidation threshold amount in stable denom.
    /// When the current collaterals value is smaller than
    /// the threshold, all collaterals will be liquidated
    pub liquidation_threshold: Uint256,
    /// Valid oracle price timeframe
    pub price_timeframe: u64,
    /// Time period that needs to pass for a bid to be activated (seconds)
    pub waiting_period: u64,
    pub overseer: HumanAddr,
    pub oraiswap_oracle: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateConfig {
        owner: Option<HumanAddr>,
        oracle_contract: Option<HumanAddr>,
        safe_ratio: Option<Decimal256>,
        bid_fee: Option<Decimal256>,
        liquidator_fee: Option<Decimal256>,
        liquidation_threshold: Option<Uint256>,
        price_timeframe: Option<u64>,
        waiting_period: Option<u64>,
        overseer: Option<HumanAddr>,
    },
    /// Owner operation to whitelist a new collateral
    WhitelistCollateral {
        collateral_token: HumanAddr,
        bid_threshold: Uint256,
        max_slot: u8,
        premium_rate_per_slot: Decimal256,
    },
    UpdateCollateralInfo {
        collateral_token: HumanAddr,
        bid_threshold: Option<Uint256>,
        max_slot: Option<u8>,
    },
    /// Withdraw a bid
    RetractBid {
        bid_idx: Uint128,
        amount: Option<Uint256>,
    },
    /// After waiting_period expires, user can activate the bid
    ActivateBids {
        collateral_token: HumanAddr,
        bids_idx: Option<Vec<Uint128>>,
    },
    /// Claim the corresponding amount of liquidated collateral
    ClaimLiquidations {
        collateral_token: HumanAddr,
        bids_idx: Option<Vec<Uint128>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Custody interface to liquidate the sent collateral
    ExecuteBid {
        liquidator: HumanAddr, // Legacy parameter, ignored
        fee_address: Option<HumanAddr>,
        repay_address: Option<HumanAddr>,
    },

    SubmitBid {
        collateral_token: HumanAddr,
        premium_slot: u8,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    LiquidationAmount {
        borrow_amount: Uint256,
        borrow_limit: Uint256,
        collaterals: TokensHuman,
        collateral_prices: Vec<Decimal256>,
    },
    CollateralInfo {
        collateral_token: HumanAddr,
    },
    Bid {
        bid_idx: Uint128,
    },
    BidsByUser {
        collateral_token: HumanAddr,
        bidder: HumanAddr,
        start_after: Option<Uint128>,
        limit: Option<u8>,
    },
    BidPool {
        collateral_token: HumanAddr,
        bid_slot: u8,
    },
    BidPoolsByCollateral {
        collateral_token: HumanAddr,
        start_after: Option<u8>,
        limit: Option<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub oracle_contract: String,
    pub stable_addr: String,
    pub safe_ratio: Decimal256,
    pub bid_fee: Decimal256,
    pub liquidator_fee: Decimal256,
    pub liquidation_threshold: Uint256,
    pub price_timeframe: u64,
    pub waiting_period: u64,
    pub overseer: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationAmountResponse {
    pub collaterals: TokensHuman,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidResponse {
    pub idx: Uint128,
    pub collateral_token: String,
    pub premium_slot: u8,
    pub bidder: String,
    pub amount: Uint256,
    pub product_snapshot: Decimal256,
    pub sum_snapshot: Decimal256,
    pub pending_liquidated_collateral: Uint256,
    pub wait_end: Option<u64>,
    pub epoch_snapshot: Uint128,
    pub scale_snapshot: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidsResponse {
    pub bids: Vec<BidResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidPoolResponse {
    pub sum_snapshot: Decimal256,
    pub product_snapshot: Decimal256,
    pub total_bid_amount: Uint256,
    pub premium_rate: Decimal256,
    pub current_epoch: Uint128,
    pub current_scale: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollateralInfoResponse {
    pub collateral_token: String,
    pub bid_threshold: Uint256,
    pub max_slot: u8,
    pub premium_rate_per_slot: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidPoolsResponse {
    pub bid_pools: Vec<BidPoolResponse>,
}
