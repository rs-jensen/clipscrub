mod config;
mod rules;
mod clipboard;
mod url_cleaner;
mod tui;
mod utils;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::clipboard::ClipboardWorker;
use crate::config::Config;
use crate::tui::Tui;

#[derive(Clone, Debug)]
pub struct CleanEvent {
    pub original: String,
    pub cleaned: String,
    pub domain: String,
    pub removed_params: Vec<String>,
    pub timestamp: Instant,
}

#[derive(Default, Clone, Debug)]
pub struct Stats {
    pub total_cleaned: u64,
    pub params_removed: u64,
    pub bytes_saved: u64,
    pub domains: HashMap<String, u64>,
}

pub struct App {
    pub events: Arc<Mutex<Vec<CleanEvent>>>,
    pub stats: Arc<Mutex<Stats>>,
    pub config: Arc<Config>,
    pub scroll_state: usize,
    pub show_help: bool,
    pub paused: bool,
    pub selected_tab: usize,
}

impl App {
    fn new(events: Arc<Mutex<Vec<CleanEvent>>>, stats: Arc<Mutex<Stats>>, config: Arc<Config>) -> Self {
        Self {
            events,
            stats,
            config,
            scroll_state: 0,
            show_help: false,
            paused: false,
            selected_tab: 0,
        }
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = !self.show_help,
            KeyCode::Char(' ') | KeyCode::Char('p') => self.paused = !self.paused,
            KeyCode::Tab => self.selected_tab = (self.selected_tab + 1) % 3,
            KeyCode::Down | KeyCode::Char('j') => self.scroll_state += 1,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_state > 0 {
                    self.scroll_state -= 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => self.scroll_state = 0,
            KeyCode::Char('c') => {
                if let Ok(mut events) = self.events.lock() {
                    events.clear();
                }
                self.scroll_state = 0;
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::load()?;
    
    let events = Arc::new(Mutex::new(Vec::new()));
    let stats = Arc::new(Mutex::new(Stats::default()));
    let paused = Arc::new(Mutex::new(false));

    // Mode: Daemon (no UI)
    if args.len() > 1 && (args[1] == "-d" || args[1] == "--daemon") {
        println!("Starting ClipScrub daemon...");
        let worker = ClipboardWorker::new(events, stats, config);
        worker.run(None); 
        return Ok(());
    }

    // Mode: One-shot clean
    if args.len() > 1 && args[1] == "--clean" {
        if args.len() < 3 {
            eprintln!("Usage: clipscrub --clean <url>");
            std::process::exit(1);
        }
        let url = &args[2];
        let worker = ClipboardWorker::new(events, stats, config);
        
        match worker.scrub_url(url) {
            Some((cleaned, removed, _)) => {
                println!("{}", cleaned);
                if !removed.is_empty() {
                    eprintln!("Removed: {}", removed.join(", "));
                }
            }
            None => println!("{}", url),
        }
        return Ok(());
    }

    // Mode: TUI (Default)
    let worker = ClipboardWorker::new(
        Arc::clone(&events),
        Arc::clone(&stats),
        Arc::clone(&config)
    );
    
    worker.spawn(Arc::clone(&paused));

    let mut app = App::new(events, stats, config);
    let mut tui = Tui::new()?;

    loop {
        tui.draw(&mut app)?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                    app.on_key(key.code);
                    
                    if let Ok(mut p) = paused.lock() {
                        *p = app.paused;
                    }
                }
            }
        }
    }

    Ok(())
}
