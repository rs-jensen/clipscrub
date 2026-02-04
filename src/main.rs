use arboard::Clipboard;
use crossterm::{terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, execute, event::{self, Event, KeyCode, KeyEventKind}};
use ratatui::{prelude::*, widgets::*};
use std::{
    io::{stdout, Stdout}, 
    time::{Duration, Instant}, 
    collections::HashMap, 
    sync::{Arc, Mutex, RwLock}, 
    thread, 
    fs, 
    path::PathBuf,
    borrow::Cow
};
use url::Url;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use regex::Regex;
use once_cell::sync::Lazy;

#[derive(Clone)]
struct CleanEvent {
    original: String,
    cleaned: String,
    domain: String,
    removed_params: Vec<String>,
    timestamp: Instant,
}

#[derive(Default, Clone)]
struct Stats {
    total_cleaned: u64,
    params_removed: u64,
    bytes_saved: u64,
    domains: HashMap<String, u64>,
}

#[derive(Deserialize, Serialize)]
struct Config {
    global_params: Vec<String>,
    domain_rules: HashMap<String, DomainRule>,
    whitelist_domains: Vec<String>,
    custom_patterns: Vec<String>,
    strip_fragments: bool,
    normalize_urls: bool,
    aggressive_mode: bool,
}

#[derive(Deserialize, Serialize, Clone)]
struct DomainRule {
    params: Vec<String>,
    keep_only: Option<Vec<String>>,
    strip_path_patterns: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        let mut domain_rules = HashMap::new();
        
        domain_rules.insert("youtube.com".into(), DomainRule {
            params: vec!["si".into(), "feature".into(), "pp".into(), "embeds_referring_euri".into(), "source_ve_path".into()],
            keep_only: Some(vec!["v".into(), "t".into(), "list".into(), "index".into()]),
            strip_path_patterns: None,
        });
        
        domain_rules.insert("twitter.com".into(), DomainRule {
            params: vec!["s".into(), "t".into(), "ref_src".into(), "ref_url".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("x.com".into(), DomainRule {
            params: vec!["s".into(), "t".into(), "ref_src".into(), "ref_url".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("amazon.com".into(), DomainRule {
            params: vec!["tag".into(), "linkCode".into(), "linkId".into(), "ref".into(), "ref_".into(), "pf_rd_r".into(), "pf_rd_p".into(), "pf_rd_s".into(), "pf_rd_t".into(), "pf_rd_i".into(), "pd_rd_r".into(), "pd_rd_w".into(), "pd_rd_wg".into(), "psc".into(), "content-id".into(), "crid".into(), "sprefix".into(), "th".into()],
            keep_only: None,
            strip_path_patterns: Some(vec![r"/ref=[^/\?]*".into()]),
        });
        
        domain_rules.insert("facebook.com".into(), DomainRule {
            params: vec!["fbclid".into(), "__tn__".into(), "__cft__".into(), "ref".into(), "fref".into(), "hc_ref".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("instagram.com".into(), DomainRule {
            params: vec!["igshid".into(), "igsh".into(), "utm_source".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("tiktok.com".into(), DomainRule {
            params: vec!["_t".into(), "_r".into(), "is_from_webapp".into(), "sender_device".into(), "enter_method".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("linkedin.com".into(), DomainRule {
            params: vec!["trk".into(), "trkInfo".into(), "originalReferer".into(), "upsellOrderOrigin".into(), "midToken".into(), "midSig".into(), "lipi".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("reddit.com".into(), DomainRule {
            params: vec!["share_id".into(), "utm_medium".into(), "utm_source".into(), "utm_name".into(), "context".into(), "ref".into(), "ref_source".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        domain_rules.insert("spotify.com".into(), DomainRule {
            params: vec!["si".into(), "context".into(), "nd".into()],
            keep_only: None,
            strip_path_patterns: None,
        });

        domain_rules.insert("ebay.com".into(), DomainRule {
            params: vec!["_trkparms".into(), "_trksid".into(), "amdata".into(), "mkevt".into(), "mkcid".into(), "mkrid".into(), "campid".into(), "toolid".into(), "customid".into()],
            keep_only: None,
            strip_path_patterns: None,
        });

        domain_rules.insert("aliexpress.com".into(), DomainRule {
            params: vec!["spm".into(), "algo_pvid".into(), "algo_expid".into(), "btsid".into(), "ws_ab_test".into(), "pdp_npi".into(), "scm".into(), "scm_id".into(), "scm-url".into(), "pvid".into(), "utparam".into(), "gatewayAda498".into(), "_t".into(), "sk".into(), "aff_fcid".into(), "aff_fsk".into(), "aff_platform".into(), "aff_trace_key".into(), "terminal_id".into()],
            keep_only: None,
            strip_path_patterns: None,
        });
        
        Self {
            global_params: vec![
                "utm_source".into(), "utm_medium".into(), "utm_campaign".into(), "utm_term".into(), "utm_content".into(), "utm_id".into(), "utm_source_platform".into(), "utm_creative_format".into(), "utm_marketing_tactic".into(),
                "fbclid".into(), "gclid".into(), "gclsrc".into(), "dclid".into(), "gbraid".into(), "wbraid".into(), "msclkid".into(),
                "mc_eid".into(), "mc_cid".into(),
                "oly_anon_id".into(), "oly_enc_id".into(),
                "_openstat".into(), "vero_id".into(), "vero_conv".into(),
                "affiliate".into(), "aff".into(), "partner".into(), "ref".into(),
                "click".into(), "clickid".into(), "click_id".into(),
                "hsCtaTracking".into(), "hsa_acc".into(), "hsa_cam".into(), "hsa_grp".into(), "hsa_ad".into(), "hsa_src".into(), "hsa_tgt".into(), "hsa_kw".into(), "hsa_mt".into(), "hsa_net".into(), "hsa_ver".into(),
                "mkt_tok".into(), "trk".into(), "trkid".into(),
                "s_kwcid".into(), "ef_id".into(),
                "__s".into(), "_branch_match_id".into(), "_branch_referrer".into(),
                "irclickid".into(), "irgwc".into(),
                "_ga".into(), "_gl".into(), "_hsenc".into(), "_hsmi".into(),
                "yclid".into(), "ymclid".into(),
                "wickedid".into(), "wickedsource".into(),
                "rb_clickid".into(), "sscid".into(),
                "guccounter".into(), "guce_referrer".into(), "guce_referrer_sig".into(),
                "__cf_chl_rt_tk".into(),
            ],
            domain_rules,
            whitelist_domains: vec![],
            custom_patterns: vec![
                r"^_[a-z]{2,4}$".into(),
                r"^(ttclid|twclkid|li_fat_id)$".into(),
            ],
            strip_fragments: false,
            normalize_urls: true,
            aggressive_mode: false,
        }
    }
}

struct App {
    events: Arc<Mutex<Vec<CleanEvent>>>,
    stats: Arc<Mutex<Stats>>,
    config: Arc<Config>,
    scroll_state: usize,
    show_help: bool,
    paused: bool,
    selected_tab: usize,
}

fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "clipscrub", "clipscrub") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).ok();
        config_dir.join("config.toml")
    } else {
        PathBuf::from("clipscrub.toml")
    }
}

fn load_or_create_config() -> Config {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
    }
    let config = Config::default();
    if let Ok(content) = toml::to_string_pretty(&config) {
        fs::write(&path, content).ok();
    }
    config
}

fn extract_domain(url: &Url) -> String {
    url.host_str()
        .map(|h| {
            let parts: Vec<&str> = h.split('.').collect();
            if parts.len() >= 2 {
                format!("{}.{}", parts[parts.len()-2], parts[parts.len()-1])
            } else {
                h.to_string()
            }
        })
        .unwrap_or_default()
}

static REGEX_CACHE: Lazy<RwLock<HashMap<String, Regex>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn matches_custom_pattern(param: &str, patterns: &[String]) -> bool {
    {
        let cache = REGEX_CACHE.read().unwrap();
        for pattern in patterns {
            if let Some(re) = cache.get(pattern) {
                if re.is_match(param) {
                    return true;
                }
            }
        }
    }

    let mut cache = REGEX_CACHE.write().unwrap();
    for pattern in patterns {
        if let Some(re) = cache.get(pattern) {
            if re.is_match(param) {
                return true;
            }
            continue;
        }

        if let Ok(re) = Regex::new(pattern) {
            let hit = re.is_match(param);
            cache.insert(pattern.clone(), re);
            if hit {
                return true;
            }
        }
    }

    false
}

fn scrub_url(input: &str, config: &Config) -> Option<(String, Vec<String>, String)> {
    let trimmed = input.trim();
    if !trimmed.starts_with("http") {
        return None;
    }

    let mut parsed = Url::parse(trimmed).ok()?;
    let domain = extract_domain(&parsed);
    
    if config.whitelist_domains.iter().any(|d| domain.contains(d)) {
        return None;
    }
    
    let mut removed = Vec::new();
    let mut modified = false;
    let domain_rule = config.domain_rules.get(&domain);
    
    if let Some(rule) = domain_rule {
        if let Some(ref patterns) = rule.strip_path_patterns {
            let path = parsed.path();
            let mut new_path = Cow::Borrowed(path);
            
            for pattern in patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(&new_path) {
                        new_path = Cow::Owned(re.replace_all(&new_path, "").to_string());
                        modified = true;
                    }
                }
            }
            if let Cow::Owned(p) = new_path {
                parsed.set_path(&p);
            }
        }
    }
    
    let current_query: Vec<(String, String)> = parsed.query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if !current_query.is_empty() {
        let mut new_query = Vec::with_capacity(current_query.len());
        let mut query_changed = false;

        for (k, v) in current_query {
            let dominated = k.to_lowercase();
            let mut should_remove = false;
            let mut is_whitelisted = false;

            if let Some(rule) = domain_rule {
                if let Some(ref keep) = rule.keep_only {
                    if !keep.iter().any(|allowed| allowed.to_lowercase() == dominated) {
                        should_remove = true;
                    } else {
                        is_whitelisted = true;
                    }
                } else if rule.params.iter().any(|p| p.to_lowercase() == dominated) {
                    should_remove = true;
                }
            }

            if !should_remove && !is_whitelisted {
                if config.global_params.iter().any(|p| p.to_lowercase() == dominated) {
                    should_remove = true;
                } else if matches_custom_pattern(&dominated, &config.custom_patterns) {
                    should_remove = true;
                } else if config.aggressive_mode {
                    let suspicious = ["track", "click", "ref", "campaign", "source", "aff", "partner"];
                    if suspicious.iter().any(|s| dominated.contains(s)) {
                        should_remove = true;
                    }
                }
            }

            if should_remove {
                removed.push(k);
                query_changed = true;
            } else {
                new_query.push((k, v));
            }
        }

        if query_changed {
            modified = true;
            if new_query.is_empty() {
                parsed.set_query(None);
            } else {
                let mut qp = parsed.query_pairs_mut();
                qp.clear();
                for (k, v) in new_query {
                    qp.append_pair(&k, &v);
                }
            }
        }
    }
    
    if config.strip_fragments && parsed.fragment().is_some() {
        parsed.set_fragment(None);
        modified = true;
    }
    
    if !modified {
        return None;
    }
    
    let mut result = parsed.to_string();
    if result.ends_with('?') { result.pop(); }
    
    Some((result, removed, domain))
}

fn clipboard_monitor(events: Arc<Mutex<Vec<CleanEvent>>>, stats: Arc<Mutex<Stats>>, config: Arc<Config>, paused: Arc<Mutex<bool>>) {
    let mut clip = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut last = String::new();
    
    loop {
        thread::sleep(Duration::from_millis(250));
        
        if *paused.lock().unwrap() {
            continue;
        }
        
        let txt = match clip.get_text() {
            Ok(t) => t,
            Err(_) => continue,
        };
        
        if txt == last { continue; }
        
        let trimmed = txt.trim();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            last = txt;
            continue;
        }
        
        if let Some((cleaned, removed, domain)) = scrub_url(&txt, &config) {
            if cleaned != txt {
                let evt = CleanEvent {
                    original: txt.clone(),
                    cleaned: cleaned.clone(),
                    domain: domain.clone(),
                    removed_params: removed.clone(),
                    timestamp: Instant::now(),
                };
                
                {
                    let mut ev = events.lock().unwrap();
                    ev.insert(0, evt);
                    if ev.len() > 100 { ev.truncate(100); }
                }
                
                {
                    let mut st = stats.lock().unwrap();
                    st.total_cleaned += 1;
                    st.params_removed += removed.len() as u64;
                    st.bytes_saved += (txt.len() - cleaned.len()) as u64;
                    *st.domains.entry(domain).or_insert(0) += 1;
                }
                
                let _ = clip.set_text(&cleaned);
                last = cleaned;
            } else {
                last = txt;
            }
        } else {
            last = txt;
        }
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
}

fn run_tui(app: &mut App, paused: Arc<Mutex<bool>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = setup_terminal()?;
    
    loop {
        terminal.draw(|f| draw_ui(f, app))?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                
                match key.code {
                    KeyCode::Char('q') => {
                        restore_terminal(&mut terminal);
                        return Ok(());
                    }
                    KeyCode::Char('?') | KeyCode::F(1) => app.show_help = !app.show_help,
                    KeyCode::Char(' ') | KeyCode::Char('p') => {
                        app.paused = !app.paused;
                        *paused.lock().unwrap() = app.paused;
                    }
                    KeyCode::Tab => app.selected_tab = (app.selected_tab + 1) % 3,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.scroll_state > 0 { app.scroll_state -= 1; }
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_state += 1,
                    KeyCode::Home | KeyCode::Char('g') => app.scroll_state = 0,
                    KeyCode::Char('c') => {
                        app.events.lock().unwrap().clear();
                        app.scroll_state = 0;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());
    
    let tabs = Tabs::new(vec!["Events", "Stats", "Domains"])
        .block(Block::default().borders(Borders::ALL).title(" clipscrub "))
        .select(app.selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);
    
    match app.selected_tab {
        0 => draw_events(f, app, chunks[1]),
        1 => draw_stats(f, app, chunks[1]),
        2 => draw_domains(f, app, chunks[1]),
        _ => {}
    }
    
    let status = if app.paused {
        Paragraph::new(" PAUSED | space: resume | q: quit | tab: switch | ?: help ")
            .style(Style::default().fg(Color::Yellow))
    } else {
        Paragraph::new(" ACTIVE | space: pause | q: quit | tab: switch | ?: help ")
            .style(Style::default().fg(Color::Green))
    };
    let status_block = status.block(Block::default().borders(Borders::ALL));
    f.render_widget(status_block, chunks[2]);
    
    if app.show_help {
        draw_help_popup(f);
    }
}

fn draw_events(f: &mut Frame, app: &App, area: Rect) {
    let events = app.events.lock().unwrap();
    
    let items: Vec<ListItem> = events.iter()
        .skip(app.scroll_state)
        .take(area.height as usize - 2)
        .map(|e| {
            let secs = e.timestamp.elapsed().as_secs();
            let time_str = if secs < 60 { format!("{}s", secs) } 
                          else if secs < 3600 { format!("{}m", secs/60) }
                          else { format!("{}h", secs/3600) };
            
            let removed_str = if e.removed_params.len() <= 3 {
                e.removed_params.join(", ")
            } else {
                format!("{}, +{} more", e.removed_params[..3].join(", "), e.removed_params.len() - 3)
            };
            
            let line = Line::from(vec![
                Span::styled(format!("[{}] ", time_str), Style::default().fg(Color::DarkGray)),
                Span::styled(&e.domain, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("-{}", removed_str), Style::default().fg(Color::Red)),
            ]);
            ListItem::new(line)
        })
        .collect();
    
    let title = format!(" Recent ({}) ", events.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats.lock().unwrap();
    
    let text = vec![
        Line::from(vec![
            Span::raw("URLs cleaned:      "),
            Span::styled(format!("{}", stats.total_cleaned), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Params removed:    "),
            Span::styled(format!("{}", stats.params_removed), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Bytes saved:       "),
            Span::styled(format!("{}", stats.bytes_saved), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Unique domains:    "),
            Span::styled(format!("{}", stats.domains.len()), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Config: ", Style::default().fg(Color::DarkGray)),
            Span::styled(get_config_path().display().to_string(), Style::default().fg(Color::DarkGray)),
        ]),
    ];
    
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Statistics "))
        .alignment(Alignment::Left);
    f.render_widget(para, area);
}

fn draw_domains(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats.lock().unwrap();
    
    let mut domain_vec: Vec<(&String, &u64)> = stats.domains.iter().collect();
    domain_vec.sort_by(|a, b| b.1.cmp(a.1));
    
    let items: Vec<ListItem> = domain_vec.iter()
        .take(area.height as usize - 2)
        .map(|(domain, count)| {
            let line = Line::from(vec![
                Span::styled(format!("{:>4} ", count), Style::default().fg(Color::Yellow)),
                Span::styled(*domain, Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Top Domains "));
    f.render_widget(list, area);
}

fn draw_help_popup(f: &mut Frame) {
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area);
    
    let help = vec![
        Line::from(Span::styled("Keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  q         quit"),
        Line::from("  space/p   pause/resume"),
        Line::from("  tab       switch tab"),
        Line::from("  j/↓       scroll down"),
        Line::from("  k/↑       scroll up"),
        Line::from("  g/home    scroll to top"),
        Line::from("  c         clear events"),
        Line::from("  ?/F1      toggle help"),
        Line::from(""),
        Line::from(Span::styled("How it works", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  Monitors clipboard for URLs"),
        Line::from("  Strips tracking parameters"),
        Line::from("  Replaces with clean URL"),
    ];
    
    let para = Paragraph::new(help)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .style(Style::default().bg(Color::Black));
    f.render_widget(para, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn run_daemon(events: Arc<Mutex<Vec<CleanEvent>>>, stats: Arc<Mutex<Stats>>, config: Arc<Config>) {
    let paused = Arc::new(Mutex::new(false));
    clipboard_monitor(events, stats, config, paused);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = Arc::new(load_or_create_config());
    let events = Arc::new(Mutex::new(Vec::new()));
    let stats = Arc::new(Mutex::new(Stats::default()));
    let paused = Arc::new(Mutex::new(false));
    
    if args.len() > 1 && (args[1] == "-d" || args[1] == "--daemon") {
        println!("running in daemon mode");
        println!("config: {}", get_config_path().display());
        run_daemon(events, stats, config);
        return;
    }
    
    if args.len() > 1 && (args[1] == "--config" || args[1] == "-c") {
        println!("{}", get_config_path().display());
        return;
    }
    
    if args.len() > 1 && args[1] == "--clean" {
        if args.len() < 3 {
            eprintln!("usage: clipscrub --clean <url>");
            std::process::exit(1);
        }
        let url = &args[2];
        match scrub_url(url, &config) {
            Some((cleaned, removed, _)) => {
                println!("{}", cleaned);
                if !removed.is_empty() {
                    eprintln!("removed: {}", removed.join(", "));
                }
            }
            None => println!("{}", url),
        }
        return;
    }
    
    let ev_clone = Arc::clone(&events);
    let st_clone = Arc::clone(&stats);
    let cfg_clone = Arc::clone(&config);
    let paused_clone = Arc::clone(&paused);
    
    thread::spawn(move || {
        clipboard_monitor(ev_clone, st_clone, cfg_clone, paused_clone);
    });
    
    let mut app = App {
        events,
        stats,
        config,
        scroll_state: 0,
        show_help: false,
        paused: false,
        selected_tab: 0,
    };
    
    if let Err(e) = run_tui(&mut app, paused) {
        eprintln!("error: {}", e);
    }
}
