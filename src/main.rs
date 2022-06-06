use dotenv::dotenv;
use std::env;
use std::{ io, io::Write };

mod data;
use data::Crawler;

#[tokio::main]
async fn main() {
    let provider_url = get_provider_url();
    let mut crawler = Crawler::new(&provider_url);
    crawler.initialize().await;
    crawler.analysis_loop().await;
}

fn get_provider_url() -> String {
    dotenv().ok();

    match env::var("MAINNET_URL") {
        Ok(url) => return url,
        Err(_) => {
            print!("Please input your mainnet provider url: ");
            io::stdout().flush().unwrap();
            let mut url = String::new();
            io::stdin()
                .read_line(&mut url)
                .expect("Error reading user input.");

            return url;
        }
    }
}
