use reqwest::Client;

pub async fn fetch() -> Result<String, reqwest::Error> {
    let client = Client::new();
    let resp: serde_json::Value = client
        .get("https://cataas.com/cat?json=true")
        .send()
        .await?
        .json()
        .await?;

    let url = resp["url"].as_str().unwrap_or("/cat").to_string();

    Ok(url)
}
