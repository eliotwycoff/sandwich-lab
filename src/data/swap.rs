use ethers::prelude::{ EthEvent, LogMeta };
use ethers::types::{ Address, U64, U256, TxHash };
use ethers::utils::{ hex, format_units };
use ethers::core as ethers_core;
use ethers::contract as ethers_contract;

use std::convert::From;

use super::{ Token };

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Swap {
    block_number: U64,
    pub tx_hash: TxHash,
    tx_index: U64,
    in0: U256,
    in1: U256,
    out0: U256,
    out1: U256
}

impl From<(RawSwap, LogMeta)> for Swap {
    fn from(data_tuple: (RawSwap, LogMeta)) -> Self {
        Self {
            block_number: data_tuple.1.block_number,
            tx_hash: data_tuple.1.transaction_hash,
            tx_index: data_tuple.1.transaction_index,
            in0: data_tuple.0.in0,
            in1: data_tuple.0.in1,
            out0: data_tuple.0.out0,
            out1: data_tuple.0.out1
        }
    }
}

impl Swap {
    pub fn block_number(&self) -> u64 {
        self.block_number.as_u64()
    }

    pub fn tx_hash(&self) -> String {
        hex::encode(self.tx_hash)
    }

    pub fn tx_index(&self) -> u64 {
        self.tx_index.as_u64()
    }

    pub fn in0(&self, decimals: u8) -> f64 {
        Self::u256_to_f64(self.in0, decimals)
    }

    pub fn in1(&self, decimals: u8) -> f64 {
        Self::u256_to_f64(self.in1, decimals)
    }

    pub fn out0(&self, decimals: u8) -> f64 {
        Self::u256_to_f64(self.out0, decimals)
    }

    pub fn out1(&self, decimals: u8) -> f64 {
        Self::u256_to_f64(self.out1, decimals)
    }

    pub fn in_out_str(&self, token0: &Token, token1: &Token, input: bool) -> String {
        let amt0 = if input { self.in0(*token0.decimals()) } else { self.out0(*token0.decimals()) };
        let amt1 = if input { self.in1(*token1.decimals()) } else { self.out1(*token1.decimals()) };
        let symbol0 = token0.symbol();
        let symbol1 = token1.symbol();

        if amt0 > 0.0 && amt1 > 0.0 {
            format!("{} {} & {} {}", amt0, symbol0, amt1, symbol1)
        } else if amt0 > 0.0 {
            format!("{} {}", amt0, symbol0)
        } else if amt1 > 0.0 {
            format!("{} {}", amt1, symbol1)
        } else {
            "".to_string()
        }
    }

    pub fn info(&self, token0: &Token, token1: &Token) -> String {
        format!("Tx Hash: {}\nSwap   : {} -> {}",
            self.tx_hash(),
            self.in_out_str(token0, token1, true),
            self.in_out_str(token0, token1, false))
    }

    fn u256_to_f64(u256: U256, decimals: u8) -> f64 {
        format_units(u256, decimals as u32).unwrap().parse::<f64>().unwrap()
    }
}

#[derive(Clone, Debug, EthEvent)]
#[ethevent(name = "Swap", abi = "Swap(address,uint,uint,uint,uint,address)")]
pub struct RawSwap {
    #[ethevent(indexed)]
    pub sender: Address,
    #[ethevent(name = "amount0In")]
    pub in0: U256,
    #[ethevent(name = "amount1In")]
    pub in1: U256,
    #[ethevent(name = "amount0Out")]
    pub out0: U256,
    #[ethevent(name = "amount1Out")]
    pub out1: U256,
    #[ethevent(indexed, name = "to")]
    pub recipient: Address
}