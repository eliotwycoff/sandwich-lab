use ethers::prelude::{ Provider, Http, Middleware, Contract, Multicall, LogMeta };
use ethers::abi::{ Abi, AbiParser };
use ethers::types::{ Address };
use ethers::utils::{ hex };

use std::sync::{ Arc };
use std::vec::{ Vec };
use std::collections::{ HashMap };
use std::{ io, io::Write };
use std::error::{ Error };

use super::{ Token, Pair, Swap, RawSwap, Sandwich };

#[derive(Debug)]
pub struct Crawler<'main> {
    provider_url: &'main str,
    pair_abi: Abi,
    token_abi: Abi,
    all_pairs: Vec<Pair<'main>>,
    latest_block: Option<u64>
}

impl<'main> Crawler<'main> {
    // Construct a new crawler instance.
    pub fn new(provider_url: &'main str) -> Self {
        let pair_abi = AbiParser::default().parse_str(r#"[
                function token0() public view returns (address)
                function token1() public view returns (address)
                function getReserves() public view returns (uint112, uint112, uint32)
            ]"#).expect("unable to parse pair contract abi");

        let token_abi = AbiParser::default().parse_str(r#"[
                function name() public view returns (string)
                function symbol() public view returns (string)
                function decimals() public view returns (uint8)
            ]"#).expect("unable to parse token contract abi");

        let all_pairs = Vec::new();

        Crawler { provider_url, pair_abi, token_abi, all_pairs, latest_block: None }
    }

    pub async fn initialize(&mut self) {
        // Load data for some sample pairs to analyze.
        let pair_addresses = [ // to do: read in addresses from a config file instead of hardcoding them
            //"0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
            //"0x9928e4046d7c6513326ccea028cd3e7a91c7590a",
            //"0x21b8065d10f73ee2e260e5b47d3344d3ced7596e",
            //"0xe1573b9d29e2183b1af0e743dc2754979a40d237",
            //"0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852",
            //"0x61b62c5d56ccd158a38367ef2f539668a06356ab",
            //"0xccb63225a7b19dcf66717e4d40c9a72b39331d61",
            //"0x9c4fe5ffd9a9fc5678cfbd93aa2d4fd684b67c4c",
            //"0xd3d2e2692501a5c9ca623199d38826e513033a17",
            //"0x9fae36a18ef8ac2b43186ade5e2b07403dc742b1",
            //"0xbb2b8038a1640196fbe3e38816f3e67cba72d940",
            //"0x3041cbd36888becc7bbcbc0045e3b1f144466f5f",
            //"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11",
            //"0x11181bd3baf5ce2a478e98361985d42625de35d1",
            //"0x7a809081f991ecfe0ab2727c7e90d2ad7c2e411e",
            "0x7b73644935b8e68019ac6356c40661e1bc315860",
            //"0xab400c46c830a2f87939dcfdcbfaaadf76f35721"
        ];

        println!("\nFetching {} Pairs:\n", pair_addresses.len());

        for address in pair_addresses {
            match self.get_pair(address).await {
                Ok(pair) => {
                    println!("{}. {}", self.all_pairs.len() + 1, pair);
                    self.all_pairs.push(pair);
                },
                Err(e) => {
                    println!("An error was encountered when fetching the pair at {}:", address);
                    println!("{}", e);
                }
            };
        }

        // Get the latest block number.
        let provider = Provider::<Http>::try_from(self.provider_url.clone()).expect("unable to connect to provider");
        self.latest_block = match provider.get_block_number().await {
            Ok(num) => Some(num.as_u64()),
            Err(_) => {
                panic!("Error: latest block number unavailable.");
            }
        }
    }

    pub async fn analysis_loop(&self) {
        // Choose which pair to analyze.
        let pair_number: usize = loop {
            print!("\nChoose a pair number: ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Error reading user input.");

            match input.trim().parse::<usize>() {
                Ok(num) => {
                    if num == 0 || num > self.all_pairs.len() {
                        println!("Please choose a number from 1 to {}.", self.all_pairs.len());
                        continue
                    }

                    break num
                },
                Err(_) => {
                    println!("Please choose a number from 1 to {}.", self.all_pairs.len());
                    continue
                }
            }
        };

        // Now that a pair has been chosen, start looking for sandwiches.
        let pair = &self.all_pairs[pair_number-1];
        let mut end_block = self.latest_block.unwrap();

        let max_chunk_size = 9999;
        let target_swaps_per_chunk = 300;
        let mut chunk_size = 999;

        let provider = Provider::<Http>::try_from(self.provider_url.clone()).expect("unable to connect to provider");
        let address = pair.address().parse::<Address>().expect("unable to parse pair address");
        let contract = Contract::new(address, self.pair_abi.clone(), provider.clone());

        loop {
            println!("Fetching {} sandwiches from blocks {} to {}\n", pair.ticker(), end_block - chunk_size, end_block);

            // Get all swaps for the chosen pair and block window.
            let raw_swaps: Vec<(RawSwap, LogMeta)> = contract.event()
                .from_block::<u64>(end_block - chunk_size).to_block::<u64>(end_block)
                .query_with_meta().await
                .expect("unable to fetch 'Swap' events from pair contract");

            let total_swaps = raw_swaps.len();

            println!(" -- Statistics -- ");
            println!("Total Blocks Scanned : {}", chunk_size + 1);
            println!("Total Swaps Completed: {}", total_swaps);

            let swaps: Vec<Swap> = raw_swaps.into_iter().map(|raw_swap| Swap::from(raw_swap)).collect();

            // Group swaps by block, and filter out blocks with less than three swaps.
            let mut swaps_per_block: HashMap<u64, u64> = HashMap::new();
            let mut swaps_by_block: HashMap<u64, Vec<Swap>> = HashMap::new();

            for swap in swaps.into_iter() {
                *swaps_per_block.entry(swap.block_number()).or_insert(0) += 1;
                swaps_by_block.entry(swap.block_number()).or_insert(Vec::new()).push(swap);
            }

            for block in swaps_per_block.keys() {
                if *swaps_per_block.get(block).unwrap() < 3 {
                    swaps_by_block.remove(block);
                }
            }

            println!("Blocks with 3+ Swaps : {}", swaps_by_block.keys().len());

            // Look for and print out any sandwich trades in the blocks with at least three swaps. 
            let mut num_sandwiches = 0;
            for block in swaps_by_block.keys() {
                let mut bundle = swaps_by_block.get(block).unwrap().to_vec();
                bundle.sort_by(|a, b| a.tx_index().partial_cmp(&b.tx_index()).unwrap());

                let sandwiches = Sandwich::multiple_from(&bundle, &pair.base, &pair.quote);
                
                if sandwiches.len() > 0 {
                    if num_sandwiches == 0 {
                        println!("* Sandwiches Found! *\n");
                    }
                }

                num_sandwiches += sandwiches.len();

                for sandwich in sandwiches {
                    println!("{}\n", sandwich.info(&pair.base, &pair.quote, self.provider_url).await);
                }
            }

            // Update the block window.
            end_block -= chunk_size + 1;

            // Update the chunk size.
            let swap_density = total_swaps as f64 / chunk_size as f64;
            chunk_size = (target_swaps_per_chunk as f64 / swap_density).floor() as u64;
            chunk_size = if chunk_size > max_chunk_size { max_chunk_size } else { chunk_size };

            if num_sandwiches == 0 {
                // Let the user know that no sandwiches were found, but the search continues.
                println!("* No Sandwiches *\n");
            } else {
                // Give the user a chance to review the data and continue whenever ready.
                print!("Continue? (y/n) : ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Error reading user input.");

                match input.trim().parse::<String>() {
                    Ok(s) => {
                        match s.as_str() {
                            "y" => continue,
                            "Y" => continue,
                            "yes" => continue,
                            _ => break
                        }
                    },
                    Err(_) => break
                }
            }
        }
    }

    // Get pair metadata (includes token metadata).
    async fn get_pair(&self, pair_address: &'static str) -> Result<Pair<'static>, Box<dyn Error>> {
        let provider = Provider::<Http>::try_from(self.provider_url.clone())?;
        let address = pair_address.parse::<Address>()?;
        let client = Arc::new(provider.clone());
        let contract = Contract::<Provider<Http>>::new(address, self.pair_abi.clone(), Arc::clone(&client));

        let get_token0 = contract.method::<_, Address>("token0", ())?;
        let get_token1 = contract.method::<_, Address>("token1", ())?;

        let mut multicall = Multicall::<Provider<Http>>::new(Arc::clone(&client), None).await?;
        multicall.add_call(get_token0).add_call(get_token1);

        let (base_address, quote_address): (Address, Address) = multicall.call().await?;
        let base = self.get_token(hex::encode(base_address)).await?;
        let quote = self.get_token(hex::encode(quote_address)).await?;

        Ok(Pair::from((pair_address, base, quote)))
    }

    // Get token metadata.
    async fn get_token(&self, token_address: String) -> Result<Token, Box<dyn Error>> {
        let provider = Provider::<Http>::try_from(self.provider_url.clone())?;
        let address = token_address.parse::<Address>()?;
        let client = Arc::new(provider.clone());
        let contract = Contract::<Provider<Http>>::new(address, self.token_abi.clone(), Arc::clone(&client));

        let get_name = contract.method::<_, String>("name", ())?;
        let get_symbol = contract.method::<_, String>("symbol", ())?;
        let get_decimals = contract.method::<_, u8>("decimals", ())?;

        let mut multicall = Multicall::<Provider<Http>>::new(Arc::clone(&client), None).await?;
        multicall.add_call(get_name).add_call(get_symbol).add_call(get_decimals);

        let (name, symbol, decimals): (String, String, u8) = multicall.call().await?;

        Ok(Token::from((token_address, name, symbol, decimals)))
    }
}