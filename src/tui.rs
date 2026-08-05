use std::io::{self, stdout, Stdout};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use crate::utils::{CenteredRect, format_duration};
use crate::App;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub fn draw(&mut self, app: &mut App) -> io::Result<()> {
        self.terminal.draw(|f| ui(f, app))?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),
                 Constraint::Min(0),
                 Constraint::Length(3),
    ])
    // RETTELSE HER: f.area() -> f.size()
    .split(f.size());

    render_tabs(f, app, chunks[0]);

    match app.selected_tab {
        0 => render_events(f, app, chunks[1]),
        1 => render_stats(f, app, chunks[1]),
        2 => render_domains(f, app, chunks[1]),
        _ => {}
    }

    render_status_bar(f, app, chunks[2]);

    if app.show_help {
        render_help_popup(f);
    }
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Live Feed", "Statistics", "Top Domains"];
    let tabs = Tabs::new(titles)
    .block(Block::default().borders(Borders::ALL).title(" ClipScrub "))
    .select(app.selected_tab)
    .style(Style::default().fg(Color::DarkGray))
    .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (status_text, style) = if app.paused {
        (" PAUSED ", Style::default().bg(Color::Yellow).fg(Color::Black))
    } else {
        (" MONITORING ", Style::default().bg(Color::Green).fg(Color::Black))
    };

    let keys = " [Space] Pause | [Q] Quit | [Tab] Switch | [?] Help ";

    let line = Line::from(vec![
        Span::styled(status_text, style.add_modifier(Modifier::BOLD)),
                          Span::styled(keys, Style::default().fg(Color::DarkGray)),
    ]);

    let para = Paragraph::new(line).block(Block::default().borders(Borders::TOP));
    f.render_widget(para, area);
}

fn render_events(f: &mut Frame, app: &App, area: Rect) {
    let events = app.events.lock().unwrap();

    let items: Vec<ListItem> = events.iter()
    .skip(app.scroll_state)
    .map(|e| {
        let time_str = format_duration(e.timestamp.elapsed());

        let mut spans = vec![
            Span::styled(format!("{:>5} ", time_str), Style::default().fg(Color::DarkGray)),
         Span::styled(format!("{:<20}", e.domain), Style::default().fg(Color::Blue)),
        ];

        if e.removed_params.is_empty() {
            spans.push(Span::styled("No params", Style::default().fg(Color::DarkGray)));
        } else {
            let count = e.removed_params.len();
            let display = if count > 3 {
                format!("-{} params", count)
            } else {
                format!("-{}", e.removed_params.join(", "))
            };
            spans.push(Span::styled(display, Style::default().fg(Color::Red)));
        }

        ListItem::new(Line::from(spans))
    })
    .collect();

    let list = List::new(items)
    .block(Block::default().borders(Borders::ALL).title(format!(" Events ({}) ", events.len())))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(list, area);
}

fn render_stats(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats.lock().unwrap();

    let lines = vec![
        Line::from(vec![
            Span::raw("Total Processed: "),
                   Span::styled(stats.total_cleaned.to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("Trackers Removed: "),
                   Span::styled(stats.params_removed.to_string(), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::raw("Bandwidth Saved: "),
                   Span::styled(format!("{} bytes", stats.bytes_saved), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Active Rules: "),
                   Span::styled(app.config.domain_rules.len().to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Mode: "),
                   if app.config.aggressive_mode {
                       Span::styled("Aggressive", Style::default().fg(Color::Red))
                   } else {
                       Span::styled("Normal", Style::default().fg(Color::Green))
                   },
        ]),
    ];

    let block = Block::default().borders(Borders::ALL).title(" Statistics ");
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn render_domains(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats.lock().unwrap();
    let mut domains: Vec<_> = stats.domains.iter().collect();
    domains.sort_by(|a, b| b.1.cmp(a.1));

    let items: Vec<ListItem> = domains.iter()
    .take(50)
    .map(|(domain, count)| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:>6} ", count), Style::default().fg(Color::Yellow)),
                                 Span::styled(*domain, Style::default().fg(Color::White)),
        ]))
    })
    .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Top Domains "));
    f.render_widget(list, area);
}

fn render_help_popup(f: &mut Frame) {
    let block = Block::default().title(" Help ").borders(Borders::ALL).style(Style::default().bg(Color::DarkGray));
    // RETTELSE HER: f.area() -> f.size()
    let area = f.size().centered(60, 40);
    f.render_widget(Clear, area);

    let text = vec![
        "Navigation",
        "----------",
        "j / Down    Scroll down",
        "k / Up      Scroll up",
        "Tab         Next view",
        "Space       Pause/Resume",
        "c           Clear history",
        "q           Quit",
    ].join("\n");

    let para = Paragraph::new(text).block(block).alignment(Alignment::Center);
    f.render_widget(para, area);
}
