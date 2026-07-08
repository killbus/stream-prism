use crate::formatter::Formatter;
use crate::model::MediaFeed;
use chrono::{TimeZone, Utc};

pub struct RssFormatter;

impl Formatter for RssFormatter {
    fn format(&self, feed: &MediaFeed, host_uri: &str) -> Result<String, String> {
        let mut channel = rss::ChannelBuilder::default();
        channel
            .title(feed.title.clone())
            .link(feed.link.clone())
            .description(feed.description.clone().unwrap_or_else(|| format!("Dynamic Podcast feed for {}", feed.title)));

        if let Some(ref cover) = feed.cover_url {
            let mut image = rss::Image::default();
            image.set_url(cover.clone());
            image.set_title(feed.title.clone());
            image.set_link(feed.link.clone());
            channel.image(Some(image));
        }

        let mut rss_items = Vec::new();
        for item in &feed.items {
            let encoded_original_url = urlencoding::encode(&item.original_url);
            let resolve_url = format!("{}/resolve?url={}", host_uri, encoded_original_url);

            let enclosure = rss::EnclosureBuilder::default()
                .url(resolve_url)
                .mime_type("video/mp4".to_string())
                .length("0".to_string())
                .build();

            let guid = rss::GuidBuilder::default()
                .value(item.id.clone())
                .permalink(false)
                .build();

            let rss_item = rss::ItemBuilder::default()
                .title(Some(item.title.clone()))
                .description(item.description.clone())
                .enclosure(Some(enclosure))
                .guid(Some(guid))
                .pub_date(Some(format_rfc2822(item.pub_date)))
                .build();

            rss_items.push(rss_item);
        }

        channel.items(rss_items);
        Ok(channel.build().to_string())
    }
}

fn format_rfc2822(timestamp: u64) -> String {
    if timestamp == 0 {
        return Utc::now().to_rfc2822();
    }
    match Utc.timestamp_opt(timestamp as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.to_rfc2822(),
        _ => Utc::now().to_rfc2822(),
    }
}
