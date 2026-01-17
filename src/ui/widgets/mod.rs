pub mod hackernews;
pub mod rss;
pub mod sports;
pub mod stocks;

use crate::feeds::{FeedData, FeedFetcher};
use ratatui::{layout::Rect, Frame};

pub trait FeedWidget: Send + Sync {
    fn id(&self) -> String;
    fn title(&self) -> &str;
    fn position(&self) -> (usize, usize);
    fn render(&self, frame: &mut Frame, area: Rect, selected: bool);
    fn update_data(&mut self, data: FeedData);
    fn create_fetcher(&self) -> Box<dyn FeedFetcher>;
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
    fn set_selected(&mut self, selected: bool);
}
