use crate::core::model::{CategoryKey, ModelOut, Scores};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

static JSON_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "category": {
          "type": "string",
          "enum": [
            "screenshot_document",
            "people",
            "food_cafe",
            "nature_landscape",
            "city_street_travel",
            "pets_animals",
            "products_objects",
            "other"
          ]
        },
        "scores": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "screenshot_document": {"type": "number", "minimum": 0, "maximum": 1},
            "people": {"type": "number", "minimum": 0, "maximum": 1},
            "food_cafe": {"type": "number", "minimum": 0, "maximum": 1},
            "nature_landscape": {"type": "number", "minimum": 0, "maximum": 1},
            "city_street_travel": {"type": "number", "minimum": 0, "maximum": 1},
            "pets_animals": {"type": "number", "minimum": 0, "maximum": 1},
            "products_objects": {"type": "number", "minimum": 0, "maximum": 1},
            "other": {"type": "number", "minimum": 0, "maximum": 1}
          },
          "required": [
            "screenshot_document",
            "people",
            "food_cafe",
            "nature_landscape",
            "city_street_travel",
            "pets_animals",
            "products_objects",
            "other"
          ]
        },
        "tags_ko": {
          "type": "array",
          "minItems": 0,
          "maxItems": 12,
          "items": {"type": "string"}
        },
        "caption_ko": {"type": "string"},
        "text_in_image_ko": {"type": "string"}
      },
      "required": ["category", "scores", "tags_ko", "caption_ko", "text_in_image_ko"]
    })
});

fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix("```").unwrap_or(trimmed);
    trimmed.trim()
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    // Best-effort: find the first balanced {...} object.
    let bytes = s.as_bytes();
    let mut start: Option<usize> = None;
    let mut depth: i32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'{' {
            if start.is_none() {
                start = Some(i);
            }
            depth += 1;
        } else if b == b'}' {
            if start.is_some() {
                depth -= 1;
                if depth == 0 {
                    let st = start?;
                    return s.get(st..=i);
                }
            }
        }
    }
    None
}

fn parse_model_out(content: &str) -> Result<ModelOut> {
    let content = strip_code_fences(content);
    let candidate = extract_first_json_object(content).unwrap_or(content);
    let parsed: Value = serde_json::from_str(candidate).map_err(|e| {
        let head = content.chars().take(220).collect::<String>();
        anyhow!("parse model json: {} | head: {}", e, head)
    })?;

    fn sanitize_korean_only(s: &str) -> String {
        // Keep Hangul + whitespace + digits + basic punctuation; strip other scripts (e.g. CJK Han characters).
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            let keep = matches!(
                ch,
                '\u{1100}'..='\u{11FF}' // Hangul Jamo
                    | '\u{3130}'..='\u{318F}' // Hangul Compatibility Jamo
                    | '\u{AC00}'..='\u{D7A3}' // Hangul Syllables
                    | '0'..='9'
                    | ' ' | '\n' | '\t'
                    | '.' | ',' | '!' | '?' | ':' | ';'
                    | '-' | '_' | '/' | '\\'
                    | '(' | ')' | '[' | ']' | '{' | '}'
                    | '"' | '\'' | '“' | '”' | '’' | '‘'
                    | '·' | '…' | '—'
            );
            if keep {
                out.push(ch);
            }
        }
        out.trim().to_string()
    }

    let category_raw = parsed.get("category").and_then(|v| v.as_str());
    let scores_obj = parsed.get("scores").and_then(|v| v.as_object());

    let scores = if let Some(obj) = scores_obj {
        let scores_map: HashMap<String, f32> = obj
            .iter()
            .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f as f32)))
            .collect::<HashMap<_, _>>();
        let mut s = Scores::from_map(&scores_map);
        s = s.normalize();
        s
    } else if let Some(cat) = category_raw {
        // Fallback: if only category is present, create a one-hot style distribution.
        let mut map = HashMap::<String, f32>::new();
        for k in [
            "screenshot_document",
            "people",
            "food_cafe",
            "nature_landscape",
            "city_street_travel",
            "pets_animals",
            "products_objects",
            "other",
        ] {
            map.insert(k.to_string(), if k == cat { 1.0 } else { 0.0 });
        }
        Scores::from_map(&map)
    } else {
        return Err(anyhow!("scores missing"));
    };

    let category = if let Some(cat) = category_raw {
        CategoryKey::from(cat)
    } else {
        scores.top().0
    };

    let tags = parsed
        .get("tags_ko")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut tags = tags
        .into_iter()
        .map(|t| sanitize_korean_only(&t))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>();
    if tags.is_empty() {
        tags.push("기타".to_string());
    }

    let caption = parsed
        .get("caption_ko")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let caption = {
        let s = sanitize_korean_only(&caption);
        if s.is_empty() {
            "설명 없음".to_string()
        } else {
            s
        }
    };
    let text_in_image = parsed
        .get("text_in_image_ko")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let text_in_image = sanitize_korean_only(&text_in_image);

    Ok(ModelOut {
        category,
        scores,
        tags_ko: tags,
        caption_ko: caption,
        text_in_image_ko: text_in_image,
    })
}

pub async fn classify_image_with_options(
    base_url: &str,
    model: &str,
    think: bool,
    base64_jpeg: &str,
    cancel: &CancellationToken,
) -> Result<(ModelOut, String)> {
    if model.trim().is_empty() {
        return Err(anyhow!("ollama model is empty"));
    }
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let client = Client::new();
    async fn send_and_read(
        client: &Client,
        url: &str,
        body: &Value,
        cancel: &CancellationToken,
    ) -> Result<(reqwest::StatusCode, String)> {
        let resp = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            r = client.post(url).json(body).send() => r?
        };
        let status = resp.status();
        let text = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            t = resp.text() => t?
        };
        Ok((status, text))
    }

    let make_base_body = |with_think: bool| {
        let mut body = json!({
          "model": model,
          "stream": false,
          "options": {
            "temperature": 0
          },
          "messages": [
              {"role": "system", "content": "You are a strict JSON generator. Return ONLY a JSON object, no markdown, no prose, no code fences. IMPORTANT: For tags_ko, caption_ko, text_in_image_ko you MUST output Korean only (Hangul). Do NOT use Chinese characters(Hanja), Japanese, or English. If any non-Korean text appears in the image, translate it to Korean; if you cannot translate reliably, output an empty string for text_in_image_ko."},
              {"role": "user", "content": "Analyze the image and output JSON with EXACT keys: {\"category\": \"screenshot_document|people|food_cafe|nature_landscape|city_street_travel|pets_animals|products_objects|other\", \"scores\": {\"screenshot_document\": number, \"people\": number, \"food_cafe\": number, \"nature_landscape\": number, \"city_street_travel\": number, \"pets_animals\": number, \"products_objects\": number, \"other\": number}, \"tags_ko\": string[], \"caption_ko\": string, \"text_in_image_ko\": string}. tags_ko and caption_ko MUST be Korean(Hangul) only. scores must be between 0 and 1 and sum to 1.", "images": [base64_jpeg]}
          ]
        });
        if !with_think {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("think".to_string(), Value::Bool(false));
            }
        }
        body
    };
    let base_body = make_base_body(think);

    let try_with_schema = || {
        let mut body = base_body.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("format".to_string(), JSON_SCHEMA.clone());
        }
        body
    };

    let mut body = try_with_schema();
    let (mut status, mut text) = send_and_read(&client, &url, &body, cancel).await?;
    if !status.is_success() {
        // Some Ollama versions/models don't support JSON schema `format` on /api/chat.
        // If we detect that, retry with `"format": "json"` and then without format.
        let lowered = text.to_lowercase();
        let format_problem = lowered.contains("format")
            || lowered.contains("json schema")
            || lowered.contains("schema")
            || lowered.contains("expected")
            || lowered.contains("unknown field");

        if format_problem {
            // Try JSON mode (string format) first.
            body = base_body.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.insert("format".to_string(), Value::String("json".to_string()));
            }
            (status, text) = send_and_read(&client, &url, &body, cancel).await?;
            if !status.is_success() {
                // Finally, retry without any format.
                body = base_body.clone();
                (status, text) = send_and_read(&client, &url, &body, cancel).await?;
            }
        }
        if !status.is_success() {
            let lowered = text.to_lowercase();
            let think_unsupported = lowered.contains("unknown field")
                && (lowered.contains("think") || lowered.contains("\"think\""));
            if think_unsupported {
                // Retry without `think` for older servers.
                let base_body_no_think = make_base_body(true);
                body = base_body_no_think.clone();
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("format".to_string(), JSON_SCHEMA.clone());
                }
                (status, text) = send_and_read(&client, &url, &body, cancel).await?;
                if !status.is_success() {
                    body = base_body_no_think.clone();
                    if let Some(obj) = body.as_object_mut() {
                        obj.insert("format".to_string(), Value::String("json".to_string()));
                    }
                    (status, text) = send_and_read(&client, &url, &body, cancel).await?;
                    if !status.is_success() {
                        body = base_body_no_think;
                        (status, text) = send_and_read(&client, &url, &body, cancel).await?;
                    }
                }
            }
        }
    }
    if !status.is_success() {
        let lowered = text.to_lowercase();
        if status.as_u16() == 404 && lowered.contains("model") {
            return Err(anyhow!(
                "ollama model not found ({}). Run `ollama pull {}` then retry. raw: {}",
                model,
                model,
                text
            ));
        }
        if lowered.contains("does not support image")
            || lowered.contains("images are not supported")
        {
            return Err(anyhow!(
                "ollama model does not support images ({}). Choose a vision model (e.g. llava / qwen2.5vl). raw: {}",
                model,
                text
            ));
        }
        return Err(anyhow!("ollama error {}: {}", status, text));
    }

    let outer: Value = serde_json::from_str(&text)?;
    let content_str = outer
        .pointer("/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing message content"))?;

    fn truncate(s: &str, max: usize) -> String {
        if s.len() <= max {
            return s.to_string();
        }
        let mut out = s.chars().take(max).collect::<String>();
        out.push_str("\n…(truncated)…");
        out
    }

    let out = parse_model_out(content_str).or_else(|_| parse_model_out(text.trim()))?;
    let log = format!(
        "url: {url}\nmodel: {model}\nthink: {think}\n\nmessage.content:\n{content}\n",
        url = url,
        model = model,
        think = think,
        content = truncate(content_str, 20000)
    );
    Ok((out, log))
}

pub async fn classify_image_streaming_with_options<F>(
    base_url: &str,
    model: &str,
    think: bool,
    base64_jpeg: &str,
    cancel: &CancellationToken,
    mut on_delta: F,
) -> Result<(ModelOut, String)>
where
    F: FnMut(&str) + Send,
{
    if model.trim().is_empty() {
        return Err(anyhow!("ollama model is empty"));
    }
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let client = Client::new();

    async fn send_streaming(
        client: &Client,
        url: &str,
        body: &Value,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response> {
        let resp = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            r = client.post(url).json(body).send() => r?
        };
        Ok(resp)
    }

    let make_base_body = |with_think_field: bool| {
        let mut body = json!({
          "model": model,
          "stream": true,
          "options": {
            "temperature": 0
          },
          "messages": [
              {"role": "system", "content": "You are a strict JSON generator. Return ONLY a JSON object, no markdown, no prose, no code fences. IMPORTANT: For tags_ko, caption_ko, text_in_image_ko you MUST output Korean only (Hangul). Do NOT use Chinese characters(Hanja), Japanese, or English. If any non-Korean text appears in the image, translate it to Korean; if you cannot translate reliably, output an empty string for text_in_image_ko."},
              {"role": "user", "content": "Analyze the image and output JSON with EXACT keys: {\"category\": \"screenshot_document|people|food_cafe|nature_landscape|city_street_travel|pets_animals|products_objects|other\", \"scores\": {\"screenshot_document\": number, \"people\": number, \"food_cafe\": number, \"nature_landscape\": number, \"city_street_travel\": number, \"pets_animals\": number, \"products_objects\": number, \"other\": number}, \"tags_ko\": string[], \"caption_ko\": string, \"text_in_image_ko\": string}. tags_ko and caption_ko MUST be Korean(Hangul) only. scores must be between 0 and 1 and sum to 1.", "images": [base64_jpeg]}
          ]
        });
        if !with_think_field {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("think".to_string(), Value::Bool(false));
            }
        }
        body
    };
    let base_body = make_base_body(think);
    let base_body_no_think = make_base_body(true);

    async fn try_streaming_sequence(
        client: &Client,
        url: &str,
        base_body: &Value,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response> {
        // 1) JSON schema format
        let mut body = base_body.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("format".to_string(), JSON_SCHEMA.clone());
        }
        let resp = send_streaming(client, url, &body, cancel).await?;
        if resp.status().is_success() {
            return Ok(resp);
        }
        let text = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            t = resp.text() => t?
        };
        let lowered = text.to_lowercase();
        let format_problem = lowered.contains("format")
            || lowered.contains("json schema")
            || lowered.contains("schema")
            || lowered.contains("expected")
            || lowered.contains("unknown field");

        if format_problem {
            // 2) "json" mode
            let mut body = base_body.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.insert("format".to_string(), Value::String("json".to_string()));
            }
            let resp2 = send_streaming(client, url, &body, cancel).await?;
            if resp2.status().is_success() {
                return Ok(resp2);
            }
        }

        // 3) no format
        let resp3 = send_streaming(client, url, base_body, cancel).await?;
        if resp3.status().is_success() {
            return Ok(resp3);
        }
        let status3 = resp3.status();
        let text3 = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            t = resp3.text() => t?
        };
        Err(anyhow!("ollama error {}: {}", status3, text3))
    }

    // Try with think setting first, then fall back if server doesn't support `think`.
    let mut resp = match try_streaming_sequence(&client, &url, &base_body, cancel).await {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            let think_unsupported = msg.contains("unknown field")
                && (msg.contains("think") || msg.contains("\"think\""));
            if think_unsupported {
                try_streaming_sequence(&client, &url, &base_body_no_think, cancel).await?
            } else {
                return Err(e);
            }
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let text = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            t = resp.text() => t?
        };
        let lowered = text.to_lowercase();
        if status.as_u16() == 404 && lowered.contains("model") {
            return Err(anyhow!(
                "ollama model not found ({}). Run `ollama pull {}` then retry. raw: {}",
                model,
                model,
                text
            ));
        }
        if lowered.contains("does not support image")
            || lowered.contains("images are not supported")
        {
            return Err(anyhow!(
                "ollama model does not support images ({}). Choose a vision model (e.g. llava / qwen2.5vl). raw: {}",
                model,
                text
            ));
        }
        return Err(anyhow!("ollama error {}: {}", status, text));
    }

    // Parse NDJSON stream, accumulate ONLY `message.content`.
    let mut buf = String::new();
    let mut accumulated = String::new();
    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => return Err(anyhow!("canceled")),
            c = resp.chunk() => c?
        };
        let Some(chunk) = next else { break };
        let part = String::from_utf8_lossy(&chunk);
        buf.push_str(&part);
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            if line.is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let delta = v
                .pointer("/message/content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !delta.is_empty() {
                accumulated.push_str(delta);
                on_delta(delta);
            }
            let done = v.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
            if done {
                // Some servers may send a final line without '\n'; still fine.
                let out = parse_model_out(accumulated.trim())
                    .or_else(|_| parse_model_out(strip_code_fences(accumulated.trim())))?;
                let log = format!(
                    "url: {url}\nmodel: {model}\nthink: {think}\nstream: true\n\nmessage.content(accumulated):\n{content}\n",
                    url = url,
                    model = model,
                    think = think,
                    content = {
                        if accumulated.len() <= 20000 {
                            accumulated.clone()
                        } else {
                            let mut s = accumulated.chars().take(20000).collect::<String>();
                            s.push_str("\n…(truncated)…");
                            s
                        }
                    }
                );
                return Ok((out, log));
            }
        }
    }

    Err(anyhow!("ollama stream ended unexpectedly"))
}

pub async fn test_connection(base_url: &str) -> Result<String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        Ok("연결 성공".to_string())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(anyhow!("ollama error {}: {}", status, text))
    }
}

pub async fn list_models(base_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let resp = client.get(url).send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("ollama error {}: {}", status, text));
    }
    let json: Value = serde_json::from_str(&text)?;
    let models = json
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("missing models field"))?;

    let mut names = models
        .iter()
        .filter_map(|m| {
            m.get("name")
                .and_then(|v| v.as_str())
                .or_else(|| m.get("model").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>();

    names.sort();
    names.dedup();
    Ok(names)
}
