use reqwest::Client;
use std::time::Duration;
use web3::Transport;

// 自定义 Transport 实现
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    client: Client,
    url: String,
}

impl ReqwestTransport {
    pub fn new(url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            url: url.to_string(),
        }
    }
}

// 为 ReqwestTransport 实现 web3::Transport trait
#[async_trait::async_trait]
impl Transport for ReqwestTransport {
    type Out = futures::future::BoxFuture<'static, web3::Result<serde_json::Value>>;

    fn prepare(&self, method: &str, params: Vec<serde_json::Value>) -> (usize, jsonrpc_core::types::request::Call) {
        let id = 1; // Use an integer ID
        let request = jsonrpc_core::types::request::Call::MethodCall(jsonrpc_core::types::request::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.to_string(),
            params: jsonrpc_core::types::Params::Array(params),
            id: jsonrpc_core::types::Id::Num(id),
        });
        (id.try_into().unwrap(), request)
    }

    fn send(&self, _id: usize, request: jsonrpc_core::types::request::Call) -> Self::Out {
        let client = self.client.clone();
        let url = self.url.clone();
        // print!("Request: {} {:?}", url, request);
        Box::pin(async move {
            let response = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|e| web3::Error::Transport(web3::error::TransportError::Message(e.to_string())))?;

            let json = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| web3::Error::Transport(web3::error::TransportError::Message(e.to_string())))?;

            Ok(json["result"].clone())
        })
    }
}
