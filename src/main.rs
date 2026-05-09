use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Clear},
};

#[derive(PartialEq)]
enum Screen {
    Home,
    Results,
}

struct App {
    input: String,
    cursor: usize,
    screen: Screen,
    results: Vec<SearchResult>,
    selected: usize,
    running: bool,
}

#[derive(Clone)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            screen: Screen::Home,
            results: Vec::new(),
            selected: 0,
            running: true,
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new();

    while app.running {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_input(&mut app, key.code).await;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    match app.screen {
        Screen::Home => render_home(f, app, area),
        Screen::Results => render_results(f, app, area),
    }
}

fn render_home(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Percentage(35),
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ]).split(area);

    // Logo
    let logo = Paragraph::new(vec![
        Line::from(Span::styled("   ╦ ╦  ╦ ╦", Style::default().fg(Color::Cyan).bold())),
        Line::from(Span::styled("   ║ ╠══╣ ╚╗", Style::default().fg(Color::Cyan).bold())),
        Line::from(Span::styled(" ╚═╝ ╩  ╩  ╩", Style::default().fg(Color::Cyan).bold())),
        Line::from(""),
        Line::from(Span::styled("  just for vibes", Style::default().fg(Color::DarkGray))),
    ]).alignment(Alignment::Center);
    f.render_widget(logo, chunks[1]);

    // Search input
    let input_width = 50.min(area.width.saturating_sub(4));
    let input_area = Rect {
        x: area.x + (area.width.saturating_sub(input_width)) / 2,
        y: chunks[2].y,
        width: input_width,
        height: 3,
    };
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Search "));
    f.render_widget(input, input_area);

    // Cursor
    f.set_cursor_position((input_area.x + 1 + app.cursor as u16, input_area.y + 1));

    // Hint
    let hint = Paragraph::new(Span::styled(
        "Enter to search · Esc to quit",
        Style::default().fg(Color::DarkGray),
    )).alignment(Alignment::Center);
    f.render_widget(hint, chunks[3]);
}

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ]).split(area);

    // Search bar
    let input_bar = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" j4v "));
    f.render_widget(input_bar, chunks[0]);

    // Results
    let results_area = chunks[1];
    let mut lines: Vec<Line> = Vec::new();
    for (i, r) in app.results.iter().enumerate() {
        let style = if i == app.selected {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::White).bold()
        };
        let url_style = Style::default().fg(Color::Green);
        let snippet_style = Style::default().fg(Color::Gray);

        lines.push(Line::from(Span::styled(&r.title, style)));
        lines.push(Line::from(Span::styled(&r.url, url_style)));
        lines.push(Line::from(Span::styled(&r.snippet, snippet_style)));
        lines.push(Line::from(""));
    }
    if app.results.is_empty() {
        lines.push(Line::from(Span::styled("No results.", Style::default().fg(Color::DarkGray))));
    }
    let results_widget = Paragraph::new(lines);
    f.render_widget(results_widget, results_area);

    // Footer
    let footer = Paragraph::new(Span::styled(
        " ↑↓ navigate · Enter open · / new search · Esc back",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(footer, chunks[2]);
}

async fn handle_input(app: &mut App, key: KeyCode) {
    match app.screen {
        Screen::Home => match key {
            KeyCode::Esc => app.running = false,
            KeyCode::Enter => {
                if !app.input.is_empty() {
                    app.results = search(&app.input).await;
                    app.selected = 0;
                    app.screen = Screen::Results;
                }
            }
            KeyCode::Char(c) => {
                app.input.insert(app.cursor, c);
                app.cursor += 1;
            }
            KeyCode::Backspace => {
                if app.cursor > 0 {
                    app.cursor -= 1;
                    app.input.remove(app.cursor);
                }
            }
            KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
            KeyCode::Right => app.cursor = (app.cursor + 1).min(app.input.len()),
            _ => {}
        },
        Screen::Results => match key {
            KeyCode::Esc => app.screen = Screen::Home,
            KeyCode::Char('/') => {
                app.input.clear();
                app.cursor = 0;
                app.screen = Screen::Home;
            }
            KeyCode::Up => app.selected = app.selected.saturating_sub(1),
            KeyCode::Down => {
                if app.selected + 1 < app.results.len() {
                    app.selected += 1;
                }
            }
            KeyCode::Enter => {
                // TODO: browse selected URL via Worker
            }
            _ => {}
        },
    }
}

async fn search(query: &str) -> Vec<SearchResult> {
    // For now, hit DuckDuckGo HTML lite directly
    let url = format!("https://html.duckduckgo.com/html/?q={}", query);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send().await;

    let Ok(resp) = resp else { return vec![] };
    let Ok(body) = resp.text().await else { return vec![] };

    // Simple extraction from DDG HTML lite
    let mut results = Vec::new();
    for chunk in body.split("class=\"result__a\"").skip(1).take(10) {
        let title = extract_between(chunk, ">", "</a>").unwrap_or_default();
        let url = extract_between(chunk, "href=\"", "\"").unwrap_or_default();
        let snippet = chunk.split("class=\"result__snippet\"")
            .nth(1)
            .and_then(|s| extract_between(s, ">", "</"))
            .unwrap_or_default();

        if !title.is_empty() {
            results.push(SearchResult {
                title: html_decode(&title),
                url: url.to_string(),
                snippet: html_decode(&snippet),
            });
        }
    }
    results
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    let j = s[i..].find(end)? + i;
    Some(&s[i..j])
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("<b>", "")
        .replace("</b>", "")
}
