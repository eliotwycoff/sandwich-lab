use std::convert::From;
use std::fmt::{ Display, Formatter, Result as FmtResult};

use super::{ Token };

#[derive(Debug)]
pub struct Pair<'init> {
    address: &'init str,
    pub base: Token,
    pub quote: Token
}

impl<'init> Pair<'init> {
    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn ticker(&self) -> String {
        format!("{}-{}", self.base.symbol(), self.quote.symbol())
    }
}

impl<'init> From<(&'init str, Token, Token)> for Pair<'init> {
    fn from(data_tuple: (&'init str, Token, Token)) -> Self {
        Self {
            address: data_tuple.0,
            base: data_tuple.1,
            quote: data_tuple.2
        }
    }
}

impl<'init> Display for Pair<'init> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "<Pair {}-{} @ {}>", self.base.symbol(), self.quote.symbol(), self.address)
    }
}