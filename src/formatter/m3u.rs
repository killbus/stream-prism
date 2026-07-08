use crate::formatter::Formatter;
use crate::model::MediaFeed;

pub struct M3uFormatter;

impl Formatter for M3uFormatter {
    fn format(&self, feed: &MediaFeed, host_uri: &str) -> Result<String, String> {
        let mut output = String::from("#EXTM3U\n");

        for item in &feed.items {
            let encoded_url = urlencoding::encode(&item.original_url);
            let resolve_url = format!("{}/resolve?url={}", host_uri, encoded_url);
            let duration = item.duration.unwrap_or(0);
            let title = &item.title;

            output.push_str(&format!("#EXTINF:{}:{}\n{}\n", duration, title, resolve_url));
        }

        Ok(output)
    }
}