use std::time::Duration;
use tokio::time::timeout;

pub type RpcResult<T> = anyhow::Result<T>;
pub type RpcResultJson = RpcResult<serde_json::Value>;

pub async fn json_rpc_drop(url: &str) {
    http_json_rpc(url, false, None).await.unwrap_or_else(|e| {
        log::warn!("json_rpc_drop error: {} {}", url.to_string(), e);
        serde_json::Value::Null
    });
}

pub async fn json_rpc(url: &str) -> RpcResultJson {
    http_json_rpc(url, false, None).await
}

pub async fn json_rpc_post(url: &str, body: &serde_json::Value) -> RpcResultJson {
    http_json_rpc(url, true, Some(body)).await
}

pub async fn http_json_rpc(url: &str, post: bool, body: Option<&serde_json::Value>) -> RpcResultJson {
    const TIMEOUT: std::time::Duration = Duration::from_secs(60);
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT) // 设置整体超时
        .connect_timeout(TIMEOUT) // 设置连接超时
        .build()?;

    let response = timeout(
        TIMEOUT,
        if post {
            client.post(url).json(body.unwrap()).send()
        } else {
            client.get(url).send()
        },
    )
    .await??;

    if response.status() != 200 {
        return Err(anyhow::anyhow!(
            "json_rpc_post error  http_code: {} {} {} {}",
            if post { "POST" } else { "GET" },
            url,
            response.status(),
            serde_json::to_string(body.unwrap_or(&serde_json::Value::Null)).unwrap(),
        ));
    }

    let value: serde_json::Value = timeout(TIMEOUT, response.json()).await??;
    Ok(value)
}
