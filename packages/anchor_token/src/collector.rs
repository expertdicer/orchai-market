use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub gov_contract: HumanAddr, // collected rewards receiver
    pub astroport_factory: HumanAddr,
    pub anchor_token: HumanAddr,
    pub oraiswap_oracle: HumanAddr,
    pub reward_factor: Decimal,
    pub max_spread: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update config interface
    /// to enable reward_factor update
    /// ## NOTE:
    /// for updating `max spread`
    /// it should be either (true, none) or (true, "0.1")
    /// if we do not want to update it
    /// it should be (false, none)
    UpdateConfig {
        reward_factor: Option<Decimal>,
        gov_contract: Option<HumanAddr>,
        astroport_factory: Option<HumanAddr>,
        oraiswap_oracle: Option<HumanAddr>,
        max_spread: (bool, Option<Decimal>),
    },
    /// Public Message
    /// Sweep all given denom balance to ANC token
    /// and execute Distribute message
    Sweep { denom: String },
    Distribute {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub gov_contract: String, // collected rewards receiver
    pub astroport_factory: String,
    pub anchor_token: String,
    pub oraiswap_oracle: String,
    pub reward_factor: Decimal,
    pub max_spread: Option<Decimal>,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub astroport_factory: HumanAddr,
    pub max_spread: Decimal,
}
