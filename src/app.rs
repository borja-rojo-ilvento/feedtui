use crate::config::{Config, WidgetConfig};
use crate::event::{Event, EventHandler};
use crate::feeds::{FeedData, FeedMessage};
use crate::ui::widgets::{
    hackernews::HackernewsWidget, rss::RssWidget, sports::SportsWidget, stocks::StocksWidget,
    FeedWidget,
};
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct App {
    config: Config,
    widgets: Vec<Box<dyn FeedWidget>>,
    selected_widget: usize,
    should_quit: bool,
    feed_rx: mpsc::UnboundedReceiver<FeedMessage>,
    feed_tx: mpsc::UnboundedSender<FeedMessage>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (feed_tx, feed_rx) = mpsc::unbounded_channel();

        let mut widgets: Vec<Box<dyn FeedWidget>> = Vec::new();

        for widget_config in &config.widgets {
            let widget: Box<dyn FeedWidget> = match widget_config {
                WidgetConfig::Hackernews(cfg) => Box::new(HackernewsWidget::new(cfg.clone())),
                WidgetConfig::Stocks(cfg) => Box::new(StocksWidget::new(cfg.clone())),
                WidgetConfig::Rss(cfg) => Box::new(RssWidget::new(cfg.clone())),
                WidgetConfig::Sports(cfg) => Box::new(SportsWidget::new(cfg.clone())),
            };
            widgets.push(widget);
        }

        Self {
            config,
            widgets,
            selected_widget: 0,
            should_quit: false,
            feed_rx,
            feed_tx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = Self::setup_terminal()?;

        // Set up panic hook to restore terminal
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            Self::restore_terminal_panic();
            original_hook(panic);
        }));

        // Start feed fetchers
        self.start_feed_fetchers();

        // Event handler
        let tick_rate = Duration::from_millis(250);
        let mut events = EventHandler::new(tick_rate);

        // Main loop
        while !self.should_quit {
            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Handle events
            tokio::select! {
                event = events.next() => {
                    if let Ok(event) = event {
                        self.handle_event(event);
                    }
                }
                Some(msg) = self.feed_rx.recv() => {
                    self.handle_feed_message(msg);
                }
            }
        }

        Self::restore_terminal(&mut terminal)?;
        Ok(())
    }

    fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }

    fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn restore_terminal_panic() {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true
                }
                KeyCode::Char('r') => self.refresh_all(),
                KeyCode::Tab => self.next_widget(),
                KeyCode::BackTab => self.prev_widget(),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
                _ => {}
            },
            Event::Tick => {}
            Event::Resize(_, _) => {}
            Event::Mouse(_) => {}
        }
    }

    fn handle_feed_message(&mut self, msg: FeedMessage) {
        for widget in &mut self.widgets {
            if widget.id() == msg.widget_id {
                widget.update_data(msg.data.clone());
                break;
            }
        }
    }

    fn start_feed_fetchers(&self) {
        for widget in &self.widgets {
            let tx = self.feed_tx.clone();
            let widget_id = widget.id();
            let fetcher = widget.create_fetcher();
            let refresh_interval = Duration::from_secs(self.config.general.refresh_interval_secs);

            tokio::spawn(async move {
                loop {
                    match fetcher.fetch().await {
                        Ok(data) => {
                            let _ = tx.send(FeedMessage {
                                widget_id: widget_id.clone(),
                                data,
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(FeedMessage {
                                widget_id: widget_id.clone(),
                                data: FeedData::Error(e.to_string()),
                            });
                        }
                    }
                    tokio::time::sleep(refresh_interval).await;
                }
            });
        }
    }

    fn refresh_all(&self) {
        // Fetchers run continuously, so this triggers an immediate refresh
        // by restarting the fetchers (simplified for now)
    }

    fn next_widget(&mut self) {
        if !self.widgets.is_empty() {
            self.widgets[self.selected_widget].set_selected(false);
            self.selected_widget = (self.selected_widget + 1) % self.widgets.len();
            self.widgets[self.selected_widget].set_selected(true);
        }
    }

    fn prev_widget(&mut self) {
        if !self.widgets.is_empty() {
            self.widgets[self.selected_widget].set_selected(false);
            self.selected_widget = if self.selected_widget == 0 {
                self.widgets.len() - 1
            } else {
                self.selected_widget - 1
            };
            self.widgets[self.selected_widget].set_selected(true);
        }
    }

    fn scroll_down(&mut self) {
        if !self.widgets.is_empty() {
            self.widgets[self.selected_widget].scroll_down();
        }
    }

    fn scroll_up(&mut self) {
        if !self.widgets.is_empty() {
            self.widgets[self.selected_widget].scroll_up();
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Calculate grid dimensions
        let (max_row, max_col) = self.calculate_grid_dimensions();

        // Create row constraints
        let row_constraints: Vec<Constraint> =
            (0..=max_row).map(|_| Constraint::Ratio(1, (max_row + 1) as u32)).collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        // Create column constraints for each row
        for row_idx in 0..=max_row {
            let col_constraints: Vec<Constraint> =
                (0..=max_col).map(|_| Constraint::Ratio(1, (max_col + 1) as u32)).collect();

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints)
                .split(rows[row_idx]);

            // Render widgets in their positions
            for (widget_idx, widget) in self.widgets.iter().enumerate() {
                let pos = widget.position();
                if pos.0 == row_idx && pos.1 <= max_col {
                    let cell = cols[pos.1];
                    widget.render(frame, cell, widget_idx == self.selected_widget);
                }
            }
        }
    }

    fn calculate_grid_dimensions(&self) -> (usize, usize) {
        let mut max_row = 0;
        let mut max_col = 0;

        for widget in &self.widgets {
            let (row, col) = widget.position();
            max_row = max_row.max(row);
            max_col = max_col.max(col);
        }

        (max_row, max_col)
    }
}
