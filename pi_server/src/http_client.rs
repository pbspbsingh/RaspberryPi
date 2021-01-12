use reqwest::header::*;
use reqwest::{Client, ClientBuilder};

const USER_AGENT_VAL: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15";

pub fn builder() -> ClientBuilder {
    let headers = vec![(USER_AGENT, USER_AGENT_VAL.parse().unwrap())]
        .into_iter()
        .collect();

    Client::builder()
        .cookie_store(true)
        .referer(true)
        .default_headers(headers)
}
