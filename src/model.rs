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
                format!(
                    "{}:{}:{}",
                    e.key,
                    e.value,
                    e.timestamp
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                )
            })
            .collect()
    }

    pub fn from_lines(lines: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        let mut secrets = Secrets::default();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(3, ':');
            let Some(key) = parts.next() else { continue }; // splitn always yields 1
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
        Ok(secrets)
    }
}
