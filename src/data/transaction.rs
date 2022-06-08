use ethers::prelude::{ Provider, Http, Middleware };
use ethers::types::{ Transaction as Tx, TransactionReceipt as TxReceipt, TxHash };

#[derive(Debug)]
pub struct Transaction {
    pub data: Tx,
    pub receipt: TxReceipt
}

impl Transaction {
    pub async fn from_hash(provider_url: String, hash: TxHash) -> Self {
        let provider = Provider::<Http>::try_from(provider_url).expect("unable to connect to the provider");
        let data = provider.get_transaction(hash).await.expect("unable to fetch transaction").unwrap();
        let receipt = provider.get_transaction_receipt(hash).await.expect("unable to fetch transaction receipt").unwrap();

        Self { data, receipt }
    }
}