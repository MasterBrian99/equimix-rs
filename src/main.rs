use serde::Deserialize;
use tokio::fs;

mod round_robin;

#[derive(Debug, Deserialize)]
struct Config {
    #[allow(dead_code)]
    config: ConfigDetails,
    #[allow(dead_code)]
    servers: Vec<Server>,
}

#[derive(Deserialize, Debug)]
struct ConfigDetails {
    #[allow(dead_code)]
    algo: String,
}

#[derive(Deserialize, Debug)]
struct Server {
    #[allow(dead_code)]
    url: String,
}

#[tokio::main]
async fn main() {
    let contents = match fs::read_to_string("config.toml").await {
        Ok(contents) => contents,
        Err(error) => panic!("Error reading file: {:?}", error),
    };

    let cargo_toml: Config = toml::from_str(&contents).expect("Failed to deserialize Cargo.toml");

    // At this point, `contents` contains the content of the TOML file
    println!("{:?}", cargo_toml);
    println!("Hello, world!");
}
