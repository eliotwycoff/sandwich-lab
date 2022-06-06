use std::convert::From;
use std::fmt::{ Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub struct Token {
    address: String,
    name: String,
    symbol: String,
    decimals: u8
}

impl Token {
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn decimals(&self) -> &u8 {
        &self.decimals
    }
}

impl From<(String, String, String, u8)> for Token {
    fn from(data_tuple: (String, String, String, u8)) -> Self {
        Self {
            address: data_tuple.0,
            name: data_tuple.1,
            symbol: data_tuple.2,
            decimals: data_tuple.3
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "<Token :: {} ({}) @ {} ({} decimals)>", self.name, self.symbol, self.address, self.decimals)
    }
}