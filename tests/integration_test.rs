#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr, Decimal};
    use std::collections::HashMap;

    use medas_computing_contract::contract::{execute, instantiate, query};
    use medas_computing_contract::msg::{
        ExecuteMsg, InstantiateMsg, PricingTier, QueryMsg, ServiceCapability,
    };

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };

        let info = mock_info("creator", &coins(0, "umedas"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(res.attributes.len(), 3);
    }

    #[test]
    fn test_register_provider() {
        let mut deps = mock_dependencies();

        // Instantiate
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1), // 0.01
                unit: "digit".to_string(),
            },
        );

        let msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        let info = mock_info("provider1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(res.attributes[0].value, "register_provider");
    }

    #[test]
    fn test_submit_and_complete_job() {
        let mut deps = mock_dependencies();

        // Setup
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1),
                unit: "digit".to_string(),
            },
        );

        let register_msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("provider1", &[]),
            register_msg,
        )
        .unwrap();

        // Submit job
        let submit_msg = ExecuteMsg::SubmitJob {
            provider: "provider1".to_string(),
            job_type: "pi_calculation".to_string(),
            parameters: r#"{"digits":10000}"#.to_string(),
        };

        let info = mock_info("client1", &coins(1_000_000, "umedas"));
        let res = execute(deps.as_mut(), mock_env(), info, submit_msg).unwrap();

        let job_id = res
            .attributes
            .iter()
            .find(|attr| attr.key == "job_id")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap();

        // Complete job
        let complete_msg = ExecuteMsg::CompleteJob {
            job_id,
            result_hash: "abc123".to_string(),
            result_url: "https://test.com/result".to_string(),
        };

        let info = mock_info("provider1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, complete_msg).unwrap();

        assert_eq!(res.messages.len(), 2); // Community + Provider payment
    }

    #[test]
    fn test_query_providers() {
        let mut deps = mock_dependencies();

        // Setup
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1),
                unit: "digit".to_string(),
            },
        );

        let register_msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("provider1", &[]),
            register_msg,
        )
        .unwrap();

        // Query providers
        let query_msg = QueryMsg::ListProviders {
            start_after: None,
            limit: None,
        };

        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        println!("Providers response: {:?}", res);
    }
    #[test]
    fn test_complete_workflow() {
        let mut deps = mock_dependencies();

        // 1. Instantiate
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // 2. Provider registriert sich
        let mut pricing = HashMap::new();
        pricing.insert("pi_calculation".to_string(), PricingTier {
            base_price: Decimal::from_ratio(1u128, 10000u128),
            unit: "digit".to_string(),
        });

        let register = ExecuteMsg::RegisterProvider {
            name: "Berlin Node".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://berlin.test".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info("provider", &[]), register).unwrap();

        // 3. Client submitted Job
        let submit = ExecuteMsg::SubmitJob {
            provider: "provider".to_string(),
            job_type: "pi_calculation".to_string(),
            parameters: r#"{"digits":10000}"#.to_string(),
        };
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("client", &coins(1_000_000, "umedas")),
            submit,
        ).unwrap();

        let job_id: u64 = res.attributes.iter()
            .find(|a| a.key == "job_id")
            .unwrap()
            .value
            .parse()
            .unwrap();

        // 4. Provider completed Job
        let complete = ExecuteMsg::CompleteJob {
            job_id,
            result_hash: "test123".to_string(),
            result_url: "https://result.test".to_string(),
        };
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("provider", &[]),
            complete,
        ).unwrap();

        assert_eq!(res.messages.len(), 2);
    }

    #[test]
    fn test_unauthorized_completion() {
        let mut deps = mock_dependencies();

        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        let mut pricing = HashMap::new();
        pricing.insert("pi_calculation".to_string(), PricingTier {
            base_price: Decimal::percent(1),
            unit: "digit".to_string(),
        });

        let register = ExecuteMsg::RegisterProvider {
            name: "Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info("provider", &[]), register).unwrap();

        let submit = ExecuteMsg::SubmitJob {
            provider: "provider".to_string(),
            job_type: "pi_calculation".to_string(),
            parameters: "{}".to_string(),
        };
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("client", &coins(1_000_000, "umedas")),
            submit,
        ).unwrap();

        let job_id: u64 = res.attributes.iter()
            .find(|a| a.key == "job_id")
            .unwrap()
            .value
            .parse()
            .unwrap();

        let complete = ExecuteMsg::CompleteJob {
            job_id,
            result_hash: "test".to_string(),
            result_url: "test".to_string(),
        };
        
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("wrong_provider", &[]),
            complete,
        ).unwrap_err();

        assert!(matches!(err, medas_computing_contract::ContractError::Unauthorized {}));
    }
}
