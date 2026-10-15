use super::{Cancellation, Translator};
use crate::transport::{Client, HttpFailure, Request};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;

// Secrets stay in the provider instance, never the shared Agent or cache metadata.
pub struct MicrosoftTranslator {
    client: Client,
    key: String,
    region: String,
}
pub struct GoogleTranslator {
    client: Client,
    key: String,
}
impl MicrosoftTranslator {
    pub fn new(key: String, region: String) -> Result<Self> {
        Ok(Self {
            client: Client::shared(),
            key,
            region,
        })
    }
}
impl GoogleTranslator {
    pub fn new(key: String) -> Result<Self> {
        Ok(Self {
            client: Client::shared(),
            key,
        })
    }
}
fn request_json(client: &Client, request: Request, cancel: &Cancellation) -> Result<Value> {
    for attempt in 0..3 {
        cancel.check()?;
        match client.bytes(request.clone().translation(cancel.clone()), 2 * 1024 * 1024) {
            Ok(bytes) => {
                return serde_json::from_slice(&bytes).context("翻译服务没有返回有效 JSON");
            }
            Err(error) => {
                let Some(http) = error.downcast_ref::<HttpFailure>() else {
                    return Err(error);
                };
                if attempt == 2 || !(http.status == 429 || (500..600).contains(&http.status)) {
                    return Err(error);
                }
                let wait = http
                    .retry_after
                    .unwrap_or(Duration::from_secs(1 << attempt));
                // Respect long Retry-After values by stopping, rather than retrying early.
                ensure!(wait <= Duration::from_secs(30), "翻译服务限流，请稍后重试");
                cancel.wait(wait)?;
            }
        }
    }
    unreachable!()
}
fn microsoft_language(language: &str) -> &str {
    match language {
        "zh" | "zh-CN" => "zh-Hans",
        "zh-TW" => "zh-Hant",
        _ => language,
    }
}
fn google_language(language: &str) -> &str {
    match language {
        "zh" | "zh-Hans" => "zh-CN",
        "zh-Hant" => "zh-TW",
        _ => language,
    }
}
impl Translator for MicrosoftTranslator {
    fn cache_id(&self) -> String {
        format!("microsoft-v3:{}", self.region.trim())
    }
    fn translate_batch(
        &self,
        texts: &[String],
        source: &str,
        target: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<String>> {
        ensure!(!self.key.trim().is_empty(), "请输入微软 Translator API Key");
        let mut request = self
            .client
            .post("https://api.cognitive.microsofttranslator.com/translate")
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .query(&[
                ("api-version", "3.0"),
                ("to", microsoft_language(target)),
                ("textType", "plain"),
            ])
            .json(&json!(
                texts
                    .iter()
                    .map(|text| json!({"Text":text}))
                    .collect::<Vec<_>>()
            ));
        if !self.region.trim().is_empty() {
            request = request.header("Ocp-Apim-Subscription-Region", self.region.trim());
        }
        if !source.is_empty() && source != "auto" {
            request = request.query(&[("from", microsoft_language(source))]);
        }
        parse_microsoft(&request_json(&self.client, request, cancel)?, texts.len())
    }
}
impl Translator for GoogleTranslator {
    fn cache_id(&self) -> String {
        "google-basic-v2".into()
    }
    fn translate_batch(
        &self,
        texts: &[String],
        source: &str,
        target: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<String>> {
        ensure!(
            !self.key.trim().is_empty(),
            "请输入 Google Cloud Translation API Key"
        );
        let mut payload = json!({"q":texts,"target":google_language(target),"format":"text"});
        if !source.is_empty() && source != "auto" {
            payload["source"] = json!(google_language(source));
        }
        let request = self
            .client
            .post("https://translation.googleapis.com/language/translate/v2")
            .header("X-Goog-Api-Key", &self.key)
            .json(&payload);
        parse_google(&request_json(&self.client, request, cancel)?, texts.len())
    }
}
fn parse_microsoft(value: &Value, count: usize) -> Result<Vec<String>> {
    let values = value.as_array().context("微软翻译响应不是列表")?;
    ensure!(values.len() == count, "微软翻译返回行数不一致");
    values
        .iter()
        .map(|value| {
            value
                .pointer("/translations/0/text")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .context("微软翻译缺少译文")
        })
        .collect()
}
fn parse_google(value: &Value, count: usize) -> Result<Vec<String>> {
    let values = value
        .pointer("/data/translations")
        .and_then(Value::as_array)
        .context("谷歌翻译缺少结果列表")?;
    ensure!(values.len() == count, "谷歌翻译返回行数不一致");
    values
        .iter()
        .map(|value| {
            value["translatedText"]
                .as_str()
                .map(decode_entities)
                .context("谷歌翻译缺少译文")
        })
        .collect()
}
fn decode_entities(source: &str) -> String {
    let mut output = String::new();
    let mut rest = source;
    while let Some(at) = rest.find('&') {
        output.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';').filter(|end| *end < 16) else {
            output.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let character = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => entity
                .strip_prefix("#x")
                .and_then(|s| u32::from_str_radix(s, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|s| s.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(character) = character {
            output.push(character);
        } else {
            output.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_responses_are_validated_and_entities_decode_once() {
        assert_eq!(
            parse_microsoft(
                &json!([{"translations":[{"text":"甲"}]},{"translations":[{"text":"乙"}]}]),
                2
            )
            .unwrap(),
            vec!["甲", "乙"]
        );
        assert!(parse_microsoft(&json!([]), 1).is_err());
        assert_eq!(
            parse_google(
                &json!({"data":{"translations":[{"translatedText":"A&#39;s &amp; B &amp;lt;"}]}}),
                1
            )
            .unwrap(),
            vec!["A's & B &lt;"]
        );
        assert!(parse_google(&json!({"data":{}}), 1).is_err());
    }
}
