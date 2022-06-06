use ethers::prelude::{ Provider, Http, Middleware };
use ethers::utils::{ format_units };

use std::vec::{ Vec };

use super::{ Token, Swap };

pub struct Sandwich<'bun> {
    pub frontrun: &'bun Swap,
    pub lunchmeat: &'bun Swap,
    pub backrun: &'bun Swap
}

impl<'bun> Sandwich<'bun> {
    pub fn multiple_from(bundle: &'bun Vec<Swap>, t0: &Token, t1: &Token) -> Vec<Self> {
        let mut sandwiches = Vec::new();
        let num_swaps = bundle.len();

        let mut i = 0;

        while i < num_swaps-2 {
            let frontrun = &bundle[i];
            let lunchmeat = &bundle[i+1];
            let backrun = &bundle[i+2];

            if Self::is_match(&frontrun, &backrun, t0, t1) {
                sandwiches.push(Sandwich { frontrun, lunchmeat, backrun });
                i += 3;
            } else {
                i += 1;
            }
        }
        
        sandwiches
    }

    pub fn revenue(&self, token0: &Token, token1: &Token) -> (f64, f64) {
        let t0d = *token0.decimals();
        let t1d = *token1.decimals();

        let t0_long_profit = self.backrun.out0(t0d) - self.frontrun.in0(t0d);
        let t1_long_profit = self.backrun.out1(t1d) - self.frontrun.in1(t1d);

        let t0_short_profit = self.frontrun.out0(t0d) - self.backrun.in0(t0d);
        let t1_short_profit = self.frontrun.out1(t1d) - self.backrun.in1(t1d);

        (t0_long_profit + t0_short_profit, t1_long_profit + t1_short_profit)
    }

    pub async fn revenue_string_with_gas(&self, token0: &Token, token1: &Token, provider_url: &str) -> String {
        let (profit0, profit1) = self.revenue(token0, token1);
        let profit0_prefix = if profit0 >= 0.0 { "+" } else { "" };
        let profit1_prefix = if profit1 >= 0.0 { "+" } else { "" };

        let (frontrun_gas, _, backrun_gas) = self.fetch_gas(provider_url).await;
        let total_gas = frontrun_gas + backrun_gas;

        format!("*** Attacker Account Δ ***\
                \n {}{} {}\
                \n {}{} {}\
                \n -{} ETH (gas)",
                profit0_prefix,
                profit0, 
                token0.symbol(), 
                profit1_prefix,
                profit1, 
                token1.symbol(),
                total_gas)
    }

    pub async fn fetch_gas(&self, provider_url: &str) -> (f64, f64, f64) {
        // Create a handle to get the frontrunning transaction.
        let url = provider_url.to_string();
        let hash = self.frontrun.tx_hash;
        let frontrun_handle = tokio::spawn(async move {
            let provider = Provider::<Http>::try_from(url).expect("unable to connect to provider");
            provider.get_transaction(hash).await.expect("unable to fetch frontrunning transaction")
        });

        // Create a handle to get the lunchmeat transaction.
        let url = provider_url.to_string();
        let hash = self.lunchmeat.tx_hash;
        let lunchmeat_handle = tokio::spawn(async move {
            let provider = Provider::<Http>::try_from(url).expect("unable to connect to provider");
            provider.get_transaction(hash).await.expect("unable to fetch lunchmeat transaction")
        });

        // Create a handle to get the backrunning transaction.
        let url = provider_url.to_string();
        let hash = self.backrun.tx_hash;
        let backrun_handle = tokio::spawn(async move {
            let provider = Provider::<Http>::try_from(url).expect("unable to connect to provider");
            provider.get_transaction(hash).await.expect("unable to fetch backrunning transaction")
        });

        // Concurrently get all the transactions back.
        let frontrun_tx = frontrun_handle.await.unwrap().unwrap();
        let lunchmeat_tx = lunchmeat_handle.await.unwrap().unwrap();
        let backrun_tx = backrun_handle.await.unwrap().unwrap();

        // Compute and format the gas costs.
        let frontrun_gas = match frontrun_tx.gas_price.unwrap().checked_mul(frontrun_tx.gas) {
            Some(value) => format_units(value, 18 as u32).unwrap().parse::<f64>().unwrap(),
            None => 0.0 as f64
        };

        let lunchmeat_gas = match lunchmeat_tx.gas_price.unwrap().checked_mul(lunchmeat_tx.gas) {
            Some(value) => format_units(value, 18 as u32).unwrap().parse::<f64>().unwrap(),
            None => 0.0 as f64
        };

        let backrun_gas = match backrun_tx.gas_price.unwrap().checked_mul(backrun_tx.gas) {
            Some(value) => format_units(value, 18 as u32).unwrap().parse::<f64>().unwrap(),
            None => 0.0 as f64
        };

        (frontrun_gas, lunchmeat_gas, backrun_gas)
    }

    pub async fn info(&self, token0: &Token, token1: &Token, provider_url: &str) -> String {
        format!("============================ Block {} =============================\
            \n\n                   -- Frontrun: Tx Idx {} -- \n{}\
            \n\n                  -- Lunchmeat: Tx Idx {} -- \n{}\
            \n\n                    -- Backrun: Tx Idx {} -- \n{}\
            \n\n{}",
            self.frontrun.block_number(),
            self.frontrun.tx_index(),
            self.frontrun.info(token0, token1),
            self.lunchmeat.tx_index(),
            self.lunchmeat.info(token0, token1),
            self.backrun.tx_index(),
            self.backrun.info(token0, token1),
            self.revenue_string_with_gas(token0, token1, provider_url).await)
    }

    // Given two swaps, determine if they match as a frontrun and backrun pair.
    fn is_match(a: &Swap, b: &Swap, t0: &Token, t1: &Token) -> bool {
        let tol = 1.01;

        let base_ratio = a.in0(*t0.decimals()) / b.out0(*t0.decimals());
        let quote_ratio = a.in1(*t1.decimals()) / b.out1(*t1.decimals());

        if 1.0/tol < base_ratio && base_ratio < tol {
            return true;
        }

        if 1.0/tol < quote_ratio && quote_ratio < tol {
            return true;
        }

        false
    }
}