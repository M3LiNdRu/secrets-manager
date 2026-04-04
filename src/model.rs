use base64::Engine;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Secrets {
    entries: BTreeMap<String, Entry>,
}

impl Secrets {
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }

    pub fn upsert(&mut self, key: String, value: String) {
        let now = Utc::now();
        let entry = Entry {
            key: key.clone(),
            value,
            timestamp: now,
        };
        self.entries.insert(key, entry);
    }

    pub fn to_lines(&self) -> Vec<String> {
        self.entries
            .values()
            .map(|e| {
                // If key or value contains ':', encode both in base64 with b64: prefix
                if e.key.contains(':') || e.value.contains(':') {
                    let key_b64 = base64::engine::general_purpose::STANDARD.encode(&e.key);
                    let value_b64 = base64::engine::general_purpose::STANDARD.encode(&e.value);
                    format!(
                        "b64:{}:{}:{}",
                        key_b64,
                        value_b64,
                        e.timestamp
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    )
                } else {
                    format!(
                        "{}:{}:{}",
                        e.key,
                        e.value,
                        e.timestamp
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    )
                }
            })
            .collect()
    }

    pub fn from_lines(lines: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        use base64::engine::Engine;

        let mut secrets = Secrets::default();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Check if this is a base64-encoded line
            if line.starts_with("b64:") {
                let rest = &line[4..]; // Skip "b64:"
                let mut parts = rest.splitn(3, ':');
                let Some(key_b64) = parts.next() else {
                    anyhow::bail!("invalid base64 record (missing key): {line}");
                };
                let Some(value_b64) = parts.next() else {
                    anyhow::bail!("invalid base64 record (missing value): {line}");
                };
                let Some(ts) = parts.next() else {
                    anyhow::bail!("invalid base64 record (missing timestamp): {line}");
                };

                let key = String::from_utf8(
                    base64::engine::general_purpose::STANDARD
                        .decode(key_b64)
                        .map_err(|e| anyhow::anyhow!("failed to decode base64 key: {e}"))?,
                )?;
                let value = String::from_utf8(
                    base64::engine::general_purpose::STANDARD
                        .decode(value_b64)
                        .map_err(|e| anyhow::anyhow!("failed to decode base64 value: {e}"))?,
                )?;

                let timestamp = chrono::DateTime::parse_from_rfc3339(ts)
                    .map_err(|e| anyhow::anyhow!("invalid timestamp '{ts}': {e}"))?
                    .with_timezone(&Utc);
                let entry = Entry {
                    key: key.clone(),
                    value,
                    timestamp,
                };
                secrets.entries.insert(key, entry);
            } else {
                // Legacy plaintext format
                let mut parts = line.splitn(3, ':');
                let Some(key) = parts.next() else { continue };
                let Some(value) = parts.next() else {
                    anyhow::bail!("invalid record (missing value): {line}");
                };
                let Some(ts) = parts.next() else {
                    anyhow::bail!("invalid record (missing timestamp): {line}");
                };
                let timestamp = chrono::DateTime::parse_from_rfc3339(ts)
                    .map_err(|e| anyhow::anyhow!("invalid timestamp '{ts}': {e}"))?
                    .with_timezone(&Utc);
                let entry = Entry {
                    key: key.to_string(),
                    value: value.to_string(),
                    timestamp,
                };
                secrets.entries.insert(key.to_string(), entry);
            }
        }
        Ok(secrets)
    }
}
