use log::error;
use std::{backtrace::Backtrace, path::Path};
use tokio::fs;

pub const TIME_SECOND: u64 = 1000;
pub const TIME_MINUTE: u64 = 60 * TIME_SECOND;
pub const TIME_HOUR: u64 = 60 * TIME_MINUTE;
pub const TIME_DAY: u64 = 24 * TIME_HOUR;

pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}

pub fn get_timestamp_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
}

pub fn get_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

pub type LoopResult = Result<(), anyhow::Error>;
pub fn set_loop<F, Fut, T>(callback: F, param: T, interval: u64)
where
    F: Fn(T) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
    T: Send + 'static + Clone,
{
    tokio::spawn(async move {
        let param = param.clone();
        loop {
            if let Err(e) = callback(param.clone()).await {
                error!("loop_error: {:?} {:?}", e, e.backtrace());
            }
            sleep_ms(interval).await;
        }
    });
}

pub fn set_loop_global<F, Fut>(callback: F, interval: u64)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            if let Err(e) = callback().await {
                error!("loop_error: {:?} {:?}", e, e.backtrace());
            }
            sleep_ms(interval).await;
        }
    });
}

pub fn set_run<F, Fut, T>(callback: F, param: T)
where
    F: Fn(T) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
    T: Send + 'static + Clone,
{
    tokio::spawn(async move {
        callback(param.clone()).await;
    });
}

pub fn set_run1<F, Fut, T, A>(callback: F, param: T, arg: A)
where
    F: Fn(T, A) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
    T: Send + 'static + Clone,
    A: Send + 'static,
{
    tokio::spawn(async move {
        callback(param.clone(), arg).await;
    });
}

pub fn log_result<T, E>(result: Result<T, E>)
where
    E: std::fmt::Display,
{
    match result {
        Result::Ok(_) => {}
        Err(e) => {
            let backtrace = Backtrace::capture();
            log::error!("rust_result_error: {} {:?}", e, backtrace);
        }
    }
}

pub async fn create_hardlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    if fs::metadata(dst.as_ref()).await.is_ok() {
        fs::remove_file(dst.as_ref()).await?;
    }

    fs::hard_link(src, dst.as_ref()).await?;
    Ok(())
}

pub fn escape_tg_markdown_v2(text: &str) -> String {
    if text.is_empty() {
        return "".to_string();
    }

    // 定义需要转义的特殊字符
    const SPECIAL_CHARS: &[char] = &['_', '*', '`', '[', ']', '(', ')', '~', '#', '+', '-', '=', '.', '\\', '>'];

    // 遍历字符串，遇到特殊字符时添加反斜杠
    let mut escaped = String::with_capacity(text.len() * 2); // 预分配空间，考虑转义后的长度
    for c in text.chars() {
        if SPECIAL_CHARS.contains(&c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }

    escaped
}

pub fn escape_tg_markdown(text: &str) -> String {
    if text.is_empty() {
        return "".to_string();
    }

    // 定义需要转义的特殊字符
    const SPECIAL_CHARS: &[char] = &['_', '*', '`', '[', ']'];

    // 遍历字符串，遇到特殊字符时添加反斜杠
    let mut escaped = String::with_capacity(text.len() * 2); // 预分配空间，考虑转义后的长度
    for c in text.chars() {
        if SPECIAL_CHARS.contains(&c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }

    escaped
}

// pub async fn tg_send_msg(msg: &str) -> anyhow::Result<()> {
//     let url = format!("https://api.telegram.org/bot{}/sendMessage", *TG_POSITION_BOT_TOKEN);
//     let client = reqwest::Client::new();
//     let body = json!({
//         "chat_id": *TG_POSITION_CHAT_ID,
//         "parse_mode": "Markdown",
//         "text": msg
//     });
//     log::info!("tg_send_msg: {}", serde_json::to_string(&body).unwrap());
//     let response = client.post(&url).json(&body).send().await?;
//     response.json::<serde_json::Value>().await?;
//     Ok(())
// }

// pub async fn tg_send_dealer_msg(msg: &str) -> anyhow::Result<()> {
//     let url = format!("https://api.telegram.org/bot{}/sendMessage", *TG_DEALER_BOT_TOKEN);
//     let client = reqwest::Client::new();
//     let body = json!({
//         "chat_id": *TG_DEALER_CHAT_ID,
//         "parse_mode": "Markdown",
//         "text": msg
//     });
//     log::info!("tg_send_dealer_msg: {}", serde_json::to_string(&body).unwrap());

//     match client.post(&url).json(&body).send().await {
//         Ok(response) => match response.json::<serde_json::Value>().await {
//             Ok(_) => Ok(()),
//             Err(e) => {
//                 log::error!("tg_send_dealer_msg_error1: {:?}", e);
//                 Err(anyhow::anyhow!(e))
//             }
//         },
//         Err(e) => {
//             log::error!("tg_send_dealer_msg_error2: {:?}", e);
//             Err(anyhow::anyhow!(e))
//         }
//     }
// }
