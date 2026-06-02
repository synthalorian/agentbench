use crate::error::{BenchError, BenchResult};
use serde::Deserialize;
use std::collections::HashMap;

/// Load a dataset from HuggingFace Hub
pub async fn load_dataset(
    dataset_name: &str,
    split: &str,
    subset: Option<&str>,
    limit: Option<usize>,
) -> BenchResult<Vec<HashMap<String, serde_json::Value>>> {
    let url = format!(
        "https://datasets-server.huggingface.co/rows?dataset={}&config={}&split={}&offset=0&limit={}",
        dataset_name,
        subset.unwrap_or("default"),
        split,
        limit.unwrap_or(100)
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(BenchError::Http)?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(BenchError::Benchmark(format!(
            "HuggingFace API error: {}",
            text
        )));
    }

    let data: HuggingFaceResponse = response.json().await?;

    let rows: Vec<HashMap<String, serde_json::Value>> = data
        .rows
        .into_iter()
        .map(|row| {
            let mut map = HashMap::new();
            for (key, value) in row.row {
                map.insert(key, value);
            }
            map
        })
        .collect();

    Ok(rows)
}

#[derive(Debug, Deserialize)]
struct HuggingFaceResponse {
    rows: Vec<HuggingFaceRow>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceRow {
    row: HashMap<String, serde_json::Value>,
}
