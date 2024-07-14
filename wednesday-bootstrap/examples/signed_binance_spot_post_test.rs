use core::time;
use std::{borrow::Cow, collections::HashMap};

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::{Method, RequestBuilder, StatusCode};
use serde::{de, Deserialize, Serialize};
use wednesday_macro::AsUrlParams;
use serde_json::map;
use thiserror::Error;
use url::UrlQuery;
use wednesday_connector::{exchange::binance::Binance, protocol::http::{parser::HttpParser, private::{encoder::HexEncoder, RequestSigner, Signer}, public::PublicNoHeaders, rest::{client::RestClient, request::RestRequest}}};
use wednesday_model::{error::SocketError, instruments::Symbol};
use wednesday_connector::protocol::http::rest::request::AsUrlParams;

struct BinanceSigner {
    api_key: String,
    secret_key: String,
}

// Configuration required to sign every Ftx `RestRequest`
struct BinanceSignConfig<'a> {
    api_key: &'a str,
    // secret_key: &'a str,
    timestamp: i64,
    method: reqwest::Method,
    // query_params: Cow<'static, str>,
    // body: Cow<'static, str>,
    query_params: String,
    body: String,
    path: Cow<'static, str>,
}

// fn map_to_query_string(query_params: Option<&HashMap<String, String>>) -> Cow<'static, str>
// {
//     match query_params {
//         Some(params) if !params.is_empty() => {
//             let query_string = params
//                 .iter()
//                 .map(|(key, value)| format!("{}={}", key, value))
//                 .collect::<Vec<String>>()
//                 .join("&");
//             Cow::Owned(query_string)
//         }
//         _ => Cow::Borrowed(""),
//     }
// }



impl Signer for BinanceSigner {
    type Config<'a> = BinanceSignConfig<'a> where Self: 'a;

    fn config<'a, Request>(
        &'a self,
        request: Request,
        _: &RequestBuilder,
    ) -> Result<Self::Config<'a>, SocketError>
    where
        Request: RestRequest,
    {
        // let query_string = request.query_params()
        //     .map_or(Cow::Borrowed(""), |params| Cow::Owned(params.to_url_params()));

        // let body_string = request.body()
        //     .map_or(Cow::Borrowed(""), |body| Cow::Owned(body.to_url_params()));

        let mut query_string = request.query_params()
            .map_or(String::new(), |params| params.to_url_params());

        let mut body_string = request.body()
            .map_or(String::new(), |body| body.to_url_params());

        let timestamp = Utc::now().timestamp_millis() - 1000;

        if !body_string.is_empty() {
            body_string.push_str("&");
            body_string.push_str(format!("timestamp={}", timestamp).as_str());
        }

        Ok(BinanceSignConfig {
            api_key: self.api_key.as_str(),
            // secret_key: self.secret_key.as_str(),
            timestamp: timestamp,
            query_params: query_string,
            body: body_string,
            method: Request::method(),
            path: request.path(),
        })
    }

    fn add_bytes_to_sign<M>(mac: &mut M, config: &Self::Config<'_>)
    where
        M: Mac,
    {
        let mut param_string = String::new();
        if config.method == reqwest::Method::GET {
            param_string.push_str(config.query_params.as_ref());
        } else if config.method == Method::POST {
            param_string.push_str(config.body.as_ref());
        } else if config.method == Method::DELETE {
            
        } else if config.method == Method::PUT{

        }
        // if !param_string.is_empty() { param_string.push_str("&"); } 
        // param_string.push_str(format!("recvWindow=5000&timestamp={}", config.timestamp).as_str());
        // param_string.push_str(format!("timestamp={}", config.timestamp).as_str());
        println!("final param string {:?}", param_string);
        mac.update(param_string.as_bytes());
    }

    fn build_signed_request(
        config: Self::Config<'_>,
        builder: RequestBuilder,
        signature: String,
    ) -> Result<reqwest::Request, SocketError> {
        builder
            .header("Content-Type", "application/json")
            .header("X-MBX-APIKEY", config.api_key)
            .query(&[
                // ("recvWindow", "5000"),
                // // ("timestamp", &config.timestamp.to_string()), 
                ("signature", &signature)])
            .body(config.body.to_string())
            .build()
            .map_err(SocketError::from)
    }
}

struct BinanceParser;

#[derive(Deserialize, Debug)]
struct BinanceApiError {
    code: i32,
    msg: String,
}

impl HttpParser for BinanceParser {
    type ApiError = BinanceApiError;
    type OutputError = ExecutionError;

    fn parse_api_error(&self, status: StatusCode, api_error: Self::ApiError) -> Self::OutputError {
        match api_error.code {
            -1104 => ExecutionError::WrongParameter(format!("It is likely that the signature was 
            included when it was not needed. Please check the API documentation: {}", api_error.msg)),
            _ => ExecutionError::Socket(SocketError::HttpResponse(status, api_error.msg)),
        }
    }
}

#[derive(Debug, Error)]
enum ExecutionError {
    #[error("request authorisation invalid: {0}")]
    Unauthorised(String),

    #[error("SocketError: {0}")]
    Socket(#[from] SocketError),

    #[error("wrong parameter: {0}")]
    WrongParameter(String),
}

struct FetchBalancesRequest {
    pub inner_query_params: BinanceAccountSnapshotQueryParams,
}

#[derive(Deserialize, Serialize, Debug, AsUrlParams)]
struct BinanceAccountSnapshotQueryParams {
    pub symbol: String,
    pub side: String,
    pub r#type: String,
    pub timeInForce: String,
    pub quantity: String,
    pub price: String,
    // #[serde(rename = "computeCommissionRates")]
    // pub commission_rates: bool,
}

impl AsUrlParams for BinanceAccountSnapshotQueryParams {
    fn to_url_params(&self) -> String {
        // println!("{:?}", self);
        // let return_string = format!(
        //     "symbol={}&side={}&type={}&timeInForce={}&quantity={}&price={}",
        //     self.symbol, self.side, self.r#type, self.timeInForce, self.quantity, self.price
        // );
        // println!("{:?}", return_string);
        // return_string
        return self.to_url_params();
    }
}

    
const DEFAULT_HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10); // Add the DEFAULT_HTTP_REQUEST_TIMEOUT constant

impl RestRequest for FetchBalancesRequest {
    type Response = BinanceSpotAccountInfo; // Define Response type
    type QueryParams = (); // FetchBalances does not require any QueryParams
    type Body = BinanceAccountSnapshotQueryParams; // FetchBalances does not require any Body

    fn path(&self) -> Cow<'static, str> {
        // Cow::Borrowed("/account")
        // Cow::Borrowed("/sapi/v1/accountSnapshot")
        // Cow::Borrowed("/api/v3/exchangeInfo")
        "/api/v3/order".into()
    }

    fn method() -> reqwest::Method {
        reqwest::Method::POST
    }
    
    fn body(&self) -> Option<&Self::Body> { 
        Some(&self.inner_query_params)
    }

    fn sign_required() -> Option<bool> {
        Some(true)
    }
}


#[derive(Deserialize, Debug)]
struct BinanceSpotAccountInfo {
    standardCommissionForOrder: CommissionRates,
    taxCommissionForOrder: CommissionRates,
    discount: DiscountInfo,
}

#[derive(Deserialize, Debug)]
struct CommissionRates {
    maker: String,
    taker: String,
}

#[derive(Deserialize, Debug)]
struct DiscountInfo {
    enabledForAccount: bool,
    enabledForSymbol: bool,
    discountAsset: String,
    discount: String,
}
#[derive(Deserialize)]
#[allow(dead_code)]
struct BinanceBalance {
    #[serde(rename = "coin")]
    symbol: Symbol,
    total: f64,
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}


/// See Barter-Execution for a comprehensive real-life example, as well as code you can use out of the
/// box to execute trades on many exchanges.
#[tokio::main]
async fn main() {
    init_logging();
    // HMAC-SHA256 encoded account API secret used for signing private http requests
    let mac: Hmac<sha2::Sha256> = Hmac::new_from_slice("TNJyGuIkhEC37ZBECtkR34MzIq0RkNirC6v3AwfVXxvK7ZU4zYejNykzFZNGx85D".as_bytes()).unwrap();

    // Build Ftx configured RequestSigner for signing http requests with hex encoding
    let request_signer = RequestSigner::new(
        BinanceSigner {
            api_key: "Z7VlP29kGkY263jwRVMHM9IxGcP8rarSxb4SxVXhxygYDZz2lEXpsejEsaR0pnpm".to_string(),
            secret_key: "TNJyGuIkhEC37ZBECtkR34MzIq0RkNirC6v3AwfVXxvK7ZU4zYejNykzFZNGx85D".to_string(),
        },
        mac,
        HexEncoder,
    );

    let request_unsigner = PublicNoHeaders{};

    // Build RestClient with Ftx configuration
    let rest_client = RestClient::new("https://testnet.binance.vision", request_signer, BinanceParser);
    // let rest_client = RestClient::new("https://testnet.binance.vision", request_unsigner, BinanceParser);

    let fetch_request = FetchBalancesRequest {
        inner_query_params: BinanceAccountSnapshotQueryParams {
            symbol: "BTCUSDT".to_string(),
            side: "SELL".to_string(),
            r#type: "LIMIT".to_string(),
            timeInForce: "GTC".to_string(),
            quantity : "0.01".to_string(),
            price: "40000.0".to_string(),
        },
    };


    // Fetch Result<FetchBalancesResponse, ExecutionError>
    let _response = rest_client.execute(fetch_request).await;

    // Print the response
    println!("{:?}", _response);

    // Handle the response
    match _response {
        Ok((status, response)) => {
            println!("Response Status: {:?}", status);
            println!("Response Body: {:?}", response);
        }
        Err(error) => {
            println!("Error: {:?}", error);
        }
    }
}
