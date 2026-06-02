use crate::db::RunSummary;

pub struct App {
    pub runs: Vec<RunSummary>,
    pub selected: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            runs: vec![],
            selected: 0,
            should_quit: false,
        }
    }

    pub fn refresh(&mut self) {
        // TODO: Load from database
    }

    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn next(&mut self) {
        if self.selected < self.runs.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn on_tick(&mut self) {
        // TODO: Poll for updates
    }
}
