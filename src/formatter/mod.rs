pub mod rss;

use crate::model::MediaFeed;

pub trait Formatter {
    /// Formats the unified MediaFeed into the target protocol's byte payload.
    fn format(&self, feed: &MediaFeed, host_uri: &str) -> Result<String, String>;
}
