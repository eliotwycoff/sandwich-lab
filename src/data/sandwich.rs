use ethers::utils::{ format_units };
use colored::*;
use digit_group::{ FormatGroup };
use std::vec::{ Vec };
use super::{ Token, Swap, Transaction };

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

    pub async fn fetch_transactions(&self, provider_url: &str) -> (Transaction, Transaction, Transaction) {
        tokio::join!(
            self.frontrun.fetch_transaction(provider_url.to_string()),
            self.lunchmeat.fetch_transaction(provider_url.to_string()),
            self.backrun.fetch_transaction(provider_url.to_string()))
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

    pub async fn revenue_string_with_gas(&self, 
        token0: &Token, 
        token1: &Token, 
        frontrun_tx: Transaction,
        backrun_tx: Transaction) -> String {

        let (profit0, profit1) = self.revenue(token0, token1);
        let profit0_prefix = if profit0 >= 0.0 { "+" } else { "" };
        let profit1_prefix = if profit1 >= 0.0 { "+" } else { "" };

        // Compute and format the gas costs.
        let frontrun_gas = match frontrun_tx.data.gas_price.unwrap().checked_mul(frontrun_tx.receipt.gas_used.unwrap()) {
            Some(value) => format_units(value, 18 as u32).unwrap().parse::<f64>().unwrap(),
            None => 0.0 as f64
        };

        let backrun_gas = match backrun_tx.data.gas_price.unwrap().checked_mul(backrun_tx.receipt.gas_used.unwrap()) {
            Some(value) => format_units(value, 18 as u32).unwrap().parse::<f64>().unwrap(),
            None => 0.0 as f64
        };

        let total_gas = frontrun_gas + backrun_gas;

        let header_string = "Attacker Account Δ".bold();
        let base_string = format!("{}{} {}", profit0_prefix, profit0.format_commas(), token0.symbol());
        let colored_base_string = if profit0 >= 0.0 { base_string.green() } else { base_string.red() };
        let quote_string = format!("{}{} {}", profit1_prefix, profit1.format_commas(), token1.symbol());
        let colored_quote_string = if profit1 >= 0.0 { quote_string.green() } else { quote_string.red() };
        let colored_gas_string = format!("-{} ETH (gas)", total_gas.format_commas()).red();

        format!("{}\
                \n {}\
                \n {}\
                \n {}",
                header_string,
                colored_base_string, 
                colored_quote_string,
                colored_gas_string)
    }

    pub async fn info(&self, token0: &Token, token1: &Token, provider_url: &str) -> String {
        let (frontrun_tx, lunchmeat_tx, backrun_tx) = self.fetch_transactions(provider_url).await;

        let header_string = format!("=========================== Block {} ============================", 
            self.frontrun.block_number().format_commas()).bold();
        let frontrun_string = format!("Frontrun  Tx (index {})", self.frontrun.tx_index()).italic();
        let lunchmeat_string = format!("Lunchmeat Tx (index {})", self.lunchmeat.tx_index()).italic();
        let backrun_string = format!("Backrun   Tx (index {})", self.backrun.tx_index()).italic();

        format!("{}\
            \n\n                        {}\
            \n{}\
            \n\n                        {}\
            \n{}\
            \n\n                        {}\
            \n{}\
            \n\n{}",
            header_string,
            frontrun_string,
            self.frontrun.info(token0, token1),
            lunchmeat_string,
            self.lunchmeat.info(token0, token1),
            backrun_string,
            self.backrun.info(token0, token1),
            self.revenue_string_with_gas(token0, token1, frontrun_tx, backrun_tx).await)
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