use crate::error::ContractError;
use crate::state::{
    read_config, read_user_reward_elem, store_config, store_user_reward_elem, Config, UserReward,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, from_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, QueryRequest, StakingMsg, StdResult, Uint128, WasmMsg,
    WasmQuery, 
};

use crate::msgs::{ClaimableResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<InitResponse, ContractError> {
    store_config(
        deps.storage,
        &Config {
            owner: msg.owner,
            native_token_denom: msg.native_token_denom,
            asset_token: msg.asset_token,
            base_apr: msg.base_apr,
            orchai_token: msg.orchai_token,
            validator_to_delegate: msg.validator_to_delegate,
        },
    )?;

    Ok(InitResponse::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, _env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            base_apr,
            asset_token,
            validator_to_delegate,
            orchai_token,
        } => update_config(
            deps,
            _env,
            info,
            owner,
            base_apr,
            asset_token,
            validator_to_delegate,
            orchai_token,
        ),
        ExecuteMsg::StakingOrai {} => staking_orai(deps, _env, info),
        ExecuteMsg::ClaimRewards { recipient } => handle_claim_reward(deps, _env, info, recipient),
        ExecuteMsg::UpdateUserReward { user } => handle_update_reward_index(deps, _env, info, user),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let contract_addr = info.sender.clone();

    match from_binary(&cw20_msg.msg.unwrap()) {
        Ok(Cw20HookMsg::WithdrawCollateral {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if contract_addr != config.asset_token {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = cw20_msg.sender;
            handle_withdraw(deps, env, info, Some(cw20_sender_addr), cw20_msg.amount.into())
        }
        _ => Err(ContractError::MissingWithdrawCollateralHook {}),
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    owner: Option<HumanAddr>,
    base_apr: Option<Decimal256>,
    asset_token: Option<HumanAddr>,
    validator_to_delegate: Option<HumanAddr>,
    orchai_token: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if HumanAddr(_info.sender.to_string()) != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    if let Some(base_apr) = base_apr {
        config.base_apr = base_apr;
    }

    if let Some(asset_token) = asset_token {
        config.asset_token = asset_token;
    }

    if let Some(validator_to_delegate) = validator_to_delegate {
        config.validator_to_delegate = validator_to_delegate;
    }

    if let Some(orchai_token) = orchai_token {
        config.orchai_token = orchai_token;
    }

    store_config(deps.storage, &config)?;
    Ok(HandleResponse::default())
}

pub fn staking_orai(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let config: Config = read_config(deps.storage)?;
    // user send orai to contract

    let amount: Uint256 = _info
        .sent_funds
        .iter()
        .find(|c| c.denom == config.native_token_denom)
        .map(|c| c.amount)
        .unwrap_or_else(Uint128::zero)
        .into();

    let mut messages: Vec<CosmosMsg> = vec![];
    // delegate orai to validator
    messages.push(CosmosMsg::Staking(StakingMsg::Delegate {
        validator: config.validator_to_delegate.clone(),
        amount: Coin {
            denom: config.native_token_denom,
            amount: amount.clone().into(),
        },
    }));

    // mint orai for user
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.asset_token,
        send: vec![],
        msg: to_binary(&Cw20HandleMsg::Mint {
            recipient: _info.sender.clone(),
            amount: amount.clone().into(),
        })?,
    }));

    // // Calculate reward

    let sender_raw = deps
        .api
        .canonical_address(&HumanAddr(_info.sender.to_string()))?;
    if read_user_reward_elem(deps.storage, &sender_raw).is_err() {
        store_user_reward_elem(
            deps.storage,
            &sender_raw,
            &UserReward {
                last_reward: Uint256::zero(),
                last_time: _env.block.time,
                amount: Uint256::zero(),
            },
        )?;
    }

    let mut user_reward: UserReward = read_user_reward_elem(deps.storage, &sender_raw)?;
    let current_time = _env.block.time;
    let year = Decimal256::from_uint256(31536000u128);
    let reward =
        user_reward.amount * Uint256::from(current_time - user_reward.last_time) * config.base_apr
            / year;
    user_reward.last_reward += reward;
    user_reward.last_time = current_time;
    user_reward.amount += amount;

    store_user_reward_elem(deps.storage, &sender_raw, &user_reward)?;

    let res = HandleResponse {
        attributes: vec![attr("action", "staking_orai"), attr("amount", amount)],
        messages: messages,
        data: None,
    };

    Ok(res)
}

pub fn handle_claim_reward(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    recipient: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let recipient = if let Some(recipient) = recipient {
        recipient
    } else {
        _info.sender.clone()
    };
    let user_raw = deps.api.canonical_address(&recipient)?;
    let mut user_reward: UserReward = read_user_reward_elem(deps.storage, &user_raw)?;

    let current_time = _env.block.time;
    let year = Decimal256::from_uint256(31536000u128);
    let reward =
        user_reward.amount * Uint256::from(current_time - user_reward.last_time) * config.base_apr
            / year;
    user_reward.last_reward += reward;
    user_reward.last_time = current_time;
    // send reward to user
    let mut messages: Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Staking(StakingMsg::Withdraw {
        validator: config.validator_to_delegate.clone(),
        recipient: Some(_env.contract.address.clone()),
    }));
    messages.push(CosmosMsg::Bank(BankMsg::Send {
        from_address: _env.contract.address.clone(),
        to_address: _info.sender,
        amount: vec![Coin {
            denom: config.native_token_denom,
            amount: user_reward.last_reward.clone().into(),
        }],
    }));

    user_reward.last_reward = Uint256::zero();
    store_user_reward_elem(deps.storage, &user_raw, &user_reward)?;

    let res = HandleResponse {
        attributes: vec![attr("claim_reward", "staking_orai")],
        messages: messages,
        data: None,
    };

    Ok(res)
}

pub fn withdraw_pos_reward(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    recipient: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    // let mut recipient_raw = _info.sender.clone();
    // if let Some(recipient) = recipient {
    //     recipient_raw = recipient;
    // }

    let recipient_raw = _env.contract.address;
    messages.push(CosmosMsg::Staking(StakingMsg::Withdraw {
        validator: config.validator_to_delegate.clone(),
        recipient: Some(recipient_raw.clone()),
    }));

    let res = HandleResponse {
        attributes: vec![
            attr("action", "withdraw"),
            attr("validator", config.validator_to_delegate),
            attr("recipient", recipient_raw),
        ],
        messages: messages,
        data: None,
    };
    Ok(res)
}

pub fn handle_update_reward_index(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    user: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let sender_raw = deps.api.canonical_address(&user)?;
    if read_user_reward_elem(deps.storage, &sender_raw).is_err() {
        store_user_reward_elem(
            deps.storage,
            &sender_raw,
            &UserReward {
                last_reward: Uint256::zero(),
                last_time: _env.block.time,
                amount: Uint256::zero(),
            },
        )?;
    }

    let mut user_reward: UserReward = read_user_reward_elem(deps.storage, &sender_raw)?;
    let current_time = _env.block.time;
    let year = Decimal256::from_uint256(31536000u128);
    let reward =
        user_reward.amount * Uint256::from(current_time - user_reward.last_time) * config.base_apr
            / year;
    user_reward.last_reward += reward;
    user_reward.last_time = current_time;

    // get current ballance
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.asset_token.clone(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: user.clone(),
        })?,
    }))?;

    let balance: Uint256 = balance.balance.into();

    user_reward.amount = balance;
    store_user_reward_elem(deps.storage, &sender_raw, &user_reward)?;
    Ok(HandleResponse::default())
}
pub fn handle_withdraw(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    recipient: Option<HumanAddr>,
    amount: Uint256,
) -> Result<HandleResponse, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let recipient = if let Some(recipient) = recipient {
        recipient
    } else {
        _info.sender.clone()
    };
    let sender_raw = deps.api.canonical_address(&recipient)?;
    if read_user_reward_elem(deps.storage, &sender_raw).is_err() {
        store_user_reward_elem(
            deps.storage,
            &sender_raw,
            &UserReward {
                last_reward: Uint256::zero(),
                last_time: _env.block.time,
                amount: Uint256::zero(),
            },
        )?;
    }

    let mut user_reward: UserReward = read_user_reward_elem(deps.storage, &sender_raw)?;
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.asset_token.clone(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: recipient.clone(),
        })?,
    }))?;

    let balance: Uint256 = balance.balance.into();

    let current_time = _env.block.time;
    let year = Decimal256::from_uint256(31536000u128);
    let reward =
        user_reward.amount * Uint256::from(current_time - user_reward.last_time) * config.base_apr
            / year;
    user_reward.last_reward += reward;
    user_reward.last_time = current_time;
    user_reward.amount = balance;

    let res = HandleResponse {
        attributes: vec![
            attr("action", "redeem_stable"),
            attr("burn_amount", amount.clone()),
            attr("redeem_amount", amount.clone()),
        ],
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.asset_token.clone(),
                send: vec![],
                msg: to_binary(&Cw20HandleMsg::Burn {
                    amount: amount.clone().into(),
                })?,
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: _env.contract.address.clone(),
                to_address: recipient.clone(),
                amount: vec![Coin {
                    denom: config.native_token_denom,
                    amount: amount.clone().into(),
                }],
            }),
        ],
        data: None,
    };
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryConfig {} => to_binary(&query_config(deps, _env)?),
        QueryMsg::Claimable { user } => to_binary(&query_claimable(deps, _env, user)?),
    }
}

pub fn query_config(deps: Deps, _env: Env) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        native_token_denom: config.native_token_denom, // "ORAI"
        asset_token: config.asset_token,
        base_apr: config.base_apr,
        orchai_token: config.orchai_token,
        validator_to_delegate: config.validator_to_delegate,
    })
}

pub fn query_claimable(deps: Deps, _env: Env, user: HumanAddr) -> StdResult<ClaimableResponse> {
    let config: Config = read_config(deps.storage)?;
    let user_raw = deps.api.canonical_address(&HumanAddr(user.to_string()))?;
    let user_reward: UserReward = read_user_reward_elem(deps.storage, &user_raw)?;

    let mut reward = user_reward.last_reward.clone();
    let current_time = _env.block.time;
    let year = Decimal256::from_uint256(31536000u128);
    reward = reward
        + user_reward.amount
            * Uint256::from(current_time - user_reward.last_time)
            * config.base_apr
            / year;

    Ok(ClaimableResponse { reward: reward })
}
