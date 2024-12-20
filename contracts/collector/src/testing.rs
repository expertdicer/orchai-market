use crate::contract::{handle, init, query_config};
use crate::mock_querier::mock_dependencies;
use anchor_token::collector::{ConfigResponse, ExecuteMsg, InstantiateMsg};
use oraiswap::asset::{Asset, AssetInfo};
use oraiswap::pair::HandleMsg as AstroportExecuteMsg;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    to_binary, Coin, ContractResult, CosmosMsg, Decimal, StdError, Uint128, WasmMsg, HumanAddr,
};
use cw20::Cw20HandleMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        astroport_factory: HumanAddr::from("astroportfactory"),
        gov_contract: HumanAddr::from("gov"),
        anchor_token: HumanAddr::from("tokenANC"),
        oraiswap_oracle: HumanAddr::from("oraiswaporacle"),
        reward_factor: Decimal::percent(90),
        max_spread: Default::default(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!("astroportfactory", config.astroport_factory.as_str());
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        astroport_factory: HumanAddr::from("astroportfactory"),
        gov_contract: HumanAddr::from("gov"),
        anchor_token: HumanAddr::from("tokenANC"),
        oraiswap_oracle: HumanAddr::from("oraiswaporacle"),
        reward_factor: Decimal::percent(90),
        max_spread: Default::default(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update reward_factor
    let info = mock_info("gov", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        reward_factor: Some(Decimal::percent(80)),
        gov_contract: Some(HumanAddr::from("new_gov")),
        astroport_factory: Some(HumanAddr::from("new_astroport_factory")),
        oraiswap_oracle: Some(HumanAddr::from("new_oraiswap_oracle")),
        max_spread: (true, Some(Decimal::percent(10))),
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value = query_config(deps.as_ref()).unwrap();
    assert_eq!(Decimal::percent(80), value.reward_factor);
    assert_eq!(value.astroport_factory, "new_astroport_factory".to_string());
    assert_eq!(value.gov_contract, "new_gov".to_string());
    assert_eq!(value.max_spread, Some(Decimal::percent(10)));

    // test max spread update
    let info = mock_info("new_gov", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        reward_factor: None,
        gov_contract: None,
        astroport_factory: None,
        oraiswap_oracle: None,
        max_spread: (true, None),
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let value = query_config(deps.as_ref()).unwrap();
    assert_eq!(Decimal::percent(80), value.reward_factor);
    assert_eq!(value.astroport_factory, HumanAddr::from("new_astroport_factory"));
    assert_eq!(value.gov_contract, HumanAddr::from("new_gov"));
    assert_eq!(value.oraiswap_oracle, HumanAddr::from("new_oraiswap_oracle"));
    assert_eq!(value.max_spread, None);

    // Unauthorized err
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        reward_factor: None,
        gov_contract: Some(HumanAddr::from("new_gov")),
        astroport_factory: Some(HumanAddr::from("new_astroport_factory")),
        oraiswap_oracle: Some(HumanAddr::from("new_oraiswap_oracle")),
        max_spread: (false, None),
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn test_sweep() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(100u128),
    }]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    deps.querier
        .with_astroport_pairs(&[(&"uusdtokenANC".to_string(), &"pairANC".to_string())]);

    let msg = InstantiateMsg {
        astroport_factory: HumanAddr::from("astroportfactory"),
        gov_contract: HumanAddr::from("gov"),
        anchor_token: HumanAddr::from("tokenANC"),
        oraiswap_oracle: HumanAddr::from("oraiswaporacle"),
        reward_factor: Decimal::percent(90),
        max_spread: Default::default(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::Sweep {
        denom: "uusd".to_string(),
    };
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // tax deduct 100 => 99
    assert_eq!(
        res.messages,
        vec![
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("pairANC"),
                msg: to_binary(&AstroportExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uusd".to_string()
                        },
                        amount: Uint128::from(99u128),
                    },
                    max_spread: Some(Decimal::percent(10)),
                    belief_price: None,
                    to: None,
                })
                .unwrap(),
                send: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(99u128),
                }],
            }),
        ]
    );
}

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"tokenANC".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
    )]);

    let msg = InstantiateMsg {
        astroport_factory: HumanAddr::from("astroportfactory"),
        gov_contract: HumanAddr::from("gov"),
        anchor_token: HumanAddr::from("tokenANC"),
        oraiswap_oracle: HumanAddr::from("oraiswaporacle"),
        reward_factor: Decimal::percent(90),
        max_spread: Default::default(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::Sweep {
        denom: "uusd".to_string(),
    };
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("tokenANC"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: HumanAddr::from("gov"),
                    amount: Uint128::from(90u128),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("tokenANC"),
                msg: to_binary(&Cw20HandleMsg::Burn {
                    amount: Uint128::from(10u128),
                })
                .unwrap(),
                send: vec![],
            }),
        ]
    )
}
