use crate::error::NexClockError;

const IP_API_URL: &str = "https://api.ipify.org";

/// Fetches the external IP address.
pub async fn fetch_external_ip() -> Result<String, NexClockError> {
    let response = reqwest::get(IP_API_URL).await?;
    let ip = response.text().await?;
    Ok(ip.trim().to_string())
}
