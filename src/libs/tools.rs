use super::global::sleep_ms;
use std::{
    io::{Read, Write},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Semaphore,
};

pub struct Tools {}

impl Tools {
    pub fn split_words(input: &str) -> Vec<String> {
        input
            .replace("([a-z])([A-Z])", "$1 $2")
            .replace("([A-Z]+)([A-Z][a-z])", "$1 $2")
            .replace("([a-zA-Z])([0-9])", "$1 $2")
            .replace("([0-9])([a-zA-Z])", "$1 $2")
            .trim()
            .split(' ')
            .map(|s| s.to_string())
            .collect()
    }

    pub fn read_file_text(path: &str) -> anyhow::Result<String> {
        let mut text = String::new();
        let mut file = std::fs::File::open(path)?;
        file.read_to_string(&mut text)?;
        Ok(text)
    }

    pub fn write_file_text(path: &str, text: &str) -> anyhow::Result<()> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(text.as_bytes())?;
        Ok(())
    }

    pub async fn async_read_file_text(path: &str) -> Option<String> {
        let mut text = String::new();
        if let Ok(mut file) = tokio::fs::File::open(path).await {
            file.read_to_string(&mut text).await.unwrap();
            Some(text)
        } else {
            None
        }
    }
    pub async fn async_read_file_json_batch<T: serde::de::DeserializeOwned>(files: &Vec<String>) -> Vec<T> {
        let mut tasks = Vec::new();
        for file in files {
            let task = async {
                let text = Tools::async_read_file_text(file).await.unwrap();
                serde_json::from_str::<T>(text.as_str()).unwrap()
            };
            tasks.push(task);
        }

        futures::future::join_all(tasks).await
    }

    pub async fn async_write_file_text(path: &str, text: &str) {
        let mut file = tokio::fs::File::create(path).await.unwrap();
        file.write_all(text.as_bytes()).await.unwrap();
    }

    pub async fn get_dir_file_count(path: &str) -> u64 {
        let mut count = 0;
        if let Ok(mut read_dir) = tokio::fs::read_dir(path).await {
            while let Some(entry) = read_dir.next_entry().await.unwrap() {
                if entry.file_type().await.unwrap().is_file() {
                    count += 1;
                }
            }
        }

        count
    }

    pub async fn get_dir_files(path: &str) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(mut read_dir) = tokio::fs::read_dir(path).await {
            while let Some(entry) = read_dir.next_entry().await.unwrap() {
                if entry.file_type().await.unwrap().is_file() {
                    files.push(entry.path().to_str().unwrap().to_string());
                }
            }
        }

        files
    }

    pub async fn binary_search<F, Fut>(left: i64, right: i64, check: F) -> i64
    where
        F: Fn(i64, i64, i64) -> Fut,
        Fut: Future<Output = bool>,
    {
        let mut left = left;
        let mut right = right;
        while left < right {
            let mid = (left + right) / 2;
            log::info!("query_address: bs {} {} {}", left, mid, right);
            if check(left, mid, right).await {
                if left == mid {
                    break;
                }
                left = mid;
            } else {
                if right == mid {
                    break;
                }
                right = mid;
            }
        }
        log::info!("query_address: result {}", left,);
        left
    }

    pub async fn parallel_traverse<T, F, Fut>(list_items: Vec<T>, max_concurrent: usize, min_delay: u64, func: F)
    where
        T: Send + std::marker::Sync + Clone + 'static,
        F: Fn(T) -> Fut + Send + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        for item in list_items {
            let _semaphore = semaphore.clone();
            let func = func.clone();
            let item = item.clone();

            sleep_ms(min_delay).await;

            let handle = tokio::spawn(async move {
                let _permit = _semaphore.clone().acquire_owned().await;
                let _ = func(item).await;
            });

            tasks.push(handle);
        }

        for task in tasks {
            let _ = task.await;
        }
    }
}
