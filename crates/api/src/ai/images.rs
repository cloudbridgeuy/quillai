use std::{sync::OnceLock, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::provider::OpenAiCompatibleConfig;
use crate::error::Error;

pub const IMAGE_MIME: &str = "image/webp";

pub struct ImageRequest<'a> {
    pub prompt: &'a str,
    pub size: &'a str,
    pub quality: &'a str,
}

#[derive(Deserialize)]
struct ImagesResponse {
    data: Vec<ImageData>,
}

#[derive(Deserialize)]
struct ImageData {
    b64_json: Option<String>,
}

fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_default()
    })
}

pub async fn generate(
    config: &OpenAiCompatibleConfig,
    request: ImageRequest<'_>,
) -> Result<Vec<u8>, Error> {
    let response = client()
        .post(format!(
            "{}/images/generations",
            config.base_url.trim_end_matches('/')
        ))
        .bearer_auth(&config.api_key)
        .json(&json!({
            "model": config.model,
            "prompt": request.prompt,
            "size": request.size,
            "quality": request.quality,
            "output_format": "webp",
        }))
        .send()
        .await
        .map_err(|error| Error::Internal(format!("failed to reach AI image provider: {error}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| Error::Internal(format!("failed to read AI image response: {error}")))?;
    if !status.is_success() {
        return Err(Error::Internal(format!(
            "AI image provider request failed ({status}): {}",
            provider_message(&body)
        )));
    }
    decode(&body)
}

fn provider_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| body.chars().take(200).collect())
}

fn decode(body: &str) -> Result<Vec<u8>, Error> {
    let missing = || Error::Internal("AI image provider returned no image".into());
    let response: ImagesResponse = serde_json::from_str(body).map_err(|_| missing())?;
    let encoded = response
        .data
        .into_iter()
        .next()
        .and_then(|image| image.b64_json)
        .ok_or_else(missing)?;
    STANDARD
        .decode(encoded.trim())
        .map_err(|_| Error::Internal("AI image provider returned invalid base64".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reads_first_b64_image_and_rejects_bad_payloads() {
        assert_eq!(
            decode(r#"{"data":[{"b64_json":"UklGRg=="},{"b64_json":"AAAA"}]}"#).unwrap(),
            b"RIFF"
        );
        for body in [
            r#"{"data":[]}"#,
            r#"{"data":[{"url":"https://example.test/a.png"}]}"#,
            r#"{"data":[{"b64_json":"not base64!"}]}"#,
            "not json",
        ] {
            assert!(decode(body).is_err(), "{body}");
        }
        assert_eq!(
            provider_message(r#"{"error":{"message":"bad size"}}"#),
            "bad size"
        );
    }
}
