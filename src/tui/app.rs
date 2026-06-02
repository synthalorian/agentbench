use crate::db::{Database, RunSummary};
use std::sync::Arc;

pub struct App {
    pub runs: Vec<RunSummary>,
    pub selected: usize,
    pub should_quit: bool,
    pub db: Option<Arc<Database>>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            runs: vec![],
            selected: 0,
            should_quit: false,
            db: None,
            loading: false,
            error: None,
        }
    }

    pub fn with_db(mut self, db: Arc<Database>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn refresh(&mut self) {
        if let Some(db) = &self.db {
            self.loading = true;
            self.error = None;
            match db.get_runs(100) {
                Ok(runs) => {
                    self.runs = runs;
                    if self.selected >= self.runs.len() && !self.runs.is_empty() {
                        self.selected = self.runs.len() - 1;
                    }
                }
                Err(e) => {
                    self.error = Some(format!("Failed to load runs: {}", e));
                }
            }
            self.loading = false;
        }
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
        // Poll for updates every tick
        self.refresh();
    }

    pub fn selected_run(&self) -> Option<&RunSummary> {
        self.runs.get(self.selected)
    }
}
