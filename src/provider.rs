use crate::config::ProviderManifest;
use crate::mapper;
use crate::model::MediaFeed;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use tracing::{info, error};

pub struct ProviderRegistry {
    providers: Vec<(ProviderManifest, Vec<Regex>)>,
    client: reqwest::Client,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            client: reqwest::Client::new(),
        }
    }

    pub fn load_from_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<(), Box<dyn std::error::Error>> {
        let dir = dir.as_ref();
        if !dir.exists() {
            fs::create_dir_all(dir)?;
            info!("Created providers directory at {:?}", dir);
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && (path.extension() == Some(std::ffi::OsStr::new("yaml")) || path.extension() == Some(std::ffi::OsStr::new("yml"))) {
                let file = fs::File::open(&path)?;
                match serde_yaml::from_reader::<_, ProviderManifest>(file) {
                    Ok(manifest) => {
                        let mut regexes = Vec::new();
                        for pattern in &manifest.capabilities.url_patterns {
                            match Regex::new(pattern) {
                                Ok(re) => regexes.push(re),
                                Err(err) => {
                                    error!("Failed to compile regex '{}' in manifest '{}': {}", pattern, manifest.id, err);
                                    return Err(err.into());
                                }
                            }
                        }
                        info!("Loaded provider manifest: {} (version {})", manifest.id, manifest.version);
                        self.providers.push((manifest, regexes));
                    }
                    Err(err) => {
                        error!("Failed to parse manifest file {:?}: {}", path, err);
                        return Err(err.into());
                    }
                }
            }
        }
        self.providers.sort_by(|a, b| b.0.priority.cmp(&a.0.priority));
        Ok(())
    }

    pub fn find_provider(&self, url: &str) -> Option<&ProviderManifest> {
        for (manifest, regexes) in &self.providers {
            for re in regexes {
                if re.is_match(url) {
                    return Some(manifest);
                }
            }
        }
        None
    }

    pub fn interpolate_value(val: &mut Value, target_url: &str) {
        match val {
            Value::String(s) => {
                if s.contains("{{target_url}}") {
                    *s = s.replace("{{target_url}}", target_url);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::interpolate_value(item, target_url);
                }
            }
            Value::Object(obj) => {
                for (_, item) in obj {
                    Self::interpolate_value(item, target_url);
                }
            }
            _ => {}
        }
    }

    pub async fn fetch_feed(&self, url: &str) -> Result<MediaFeed, Box<dyn std::error::Error>> {
        let provider = self.find_provider(url)
            .ok_or_else(|| format!("No provider found matching URL: {}", url))?;

        let mut payload = provider.actions.fetch_feed.payload.clone();
        Self::interpolate_value(&mut payload, url);

        let action_url = format!("{}{}", provider.endpoint, provider.actions.fetch_feed.path);
        info!("Sending fetch_feed request to provider: {} at {}", provider.id, action_url);

        let mut req = self.client.request(
            reqwest::Method::from_str(&provider.actions.fetch_feed.method)?,
            &action_url
        );

        if let Some(ref headers) = provider.actions.fetch_feed.headers {
            for (k, v) in headers {
                req = req.header(k, v);
            }
        }

        let res = req.json(&payload).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            error!("Provider returned error status: {}. Body: {}", status, body);
            return Err(format!("ProviderError: HTTP {}", status).into());
        }

        let raw_json: Value = res.json().await?;
        let feed = mapper::map_feed(raw_json, provider, url)?;
        Ok(feed)
    }

    pub async fn resolve_stream(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let provider = self.find_provider(url)
            .ok_or_else(|| format!("No provider found matching URL: {}", url))?;

        let mut payload = provider.actions.resolve_stream.payload.clone();
        Self::interpolate_value(&mut payload, url);

        let action_url = format!("{}{}", provider.endpoint, provider.actions.resolve_stream.path);
        info!("Sending resolve_stream request to provider: {} at {}", provider.id, action_url);

        let mut req = self.client.request(
            reqwest::Method::from_str(&provider.actions.resolve_stream.method)?,
            &action_url
        );

        if let Some(ref headers) = provider.actions.resolve_stream.headers {
            for (k, v) in headers {
                req = req.header(k, v);
            }
        }

        let res = req.json(&payload).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            error!("Provider returned error status during resolve_stream: {}. Body: {}", status, body);
            return Err(format!("ProviderError: HTTP {}", status).into());
        }

        let raw_json: Value = res.json().await?;
        let stream_url = mapper::map_stream(raw_json, provider)?;
        Ok(stream_url)
    }
}
