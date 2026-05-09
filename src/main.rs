use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Clear},
};
use tokio::sync::mpsc;

enum BgResult {
    Search(Vec<SearchResult>),
    Page(String, Vec<PageLine>),
}

#[derive(PartialEq)]
enum Screen { Home, Results, Browse }

#[derive(Clone)]
enum PageLine {
    Heading(String),
    Text(String),
    Link(String, String),
}

#[derive(Clone)]
struct SearchResult { title: String, url: String, snippet: String }

struct App {
    input: String,
    cursor: usize,
    screen: Screen,
    results: Vec<SearchResult>,
    selected: usize,
    running: bool,
    page_title: String,
    page_url: String,
    page_lines: Vec<PageLine>,
    page_scroll: u16,
    page_link_idx: usize, // currently selected link index
    status: String,
    loading: bool,
    spinner: usize,
    history: Vec<String>, // URL back-stack
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(), cursor: 0, screen: Screen::Home,
            results: Vec::new(), selected: 0, running: true,
            page_title: String::new(), page_url: String::new(),
            page_lines: Vec::new(), page_scroll: 0, page_link_idx: 0,
            status: String::new(), loading: false, spinner: 0,
            history: Vec::new(),
        }
    }
    fn spinner_char(&self) -> &str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[self.spinner % FRAMES.len()]
    }
    fn page_links(&self) -> Vec<(usize, &str)> {
        self.page_lines.iter().enumerate().filter_map(|(i, l)| {
            if let PageLine::Link(_, href) = l { Some((i, href.as_str())) } else { None }
        }).collect()
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Agent mode: j4v --url <url> [--json]
    if args.iter().any(|a| a == "--url") {
        let url = args.iter().position(|a| a == "--url")
            .and_then(|i| args.get(i + 1))
            .expect("Usage: j4v --url <url> [--json]");
        let json_mode = args.iter().any(|a| a == "--json");
        let (title, lines) = browse(url).await;

        if json_mode {
            let elements: Vec<serde_json::Value> = lines.iter().map(|l| match l {
                PageLine::Heading(t) => serde_json::json!({"type": "heading", "text": t}),
                PageLine::Text(t) => serde_json::json!({"type": "text", "content": t}),
                PageLine::Link(text, href) => serde_json::json!({"type": "link", "text": text, "href": href}),
            }).collect();
            let out = serde_json::json!({"title": title, "url": url, "elements": elements});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            if !title.is_empty() { println!("# {}\n", title); }
            for line in &lines {
                match line {
                    PageLine::Heading(t) => println!("\n## {}", t),
                    PageLine::Text(t) => println!("{}", t),
                    PageLine::Link(text, href) => println!("  → {} [{}]", text, href),
                }
            }
        }
        return Ok(());
    }

    // Search mode: j4v --search <query> [--json]
    if args.iter().any(|a| a == "--search") {
        let query = args.iter().position(|a| a == "--search")
            .and_then(|i| args.get(i + 1))
            .expect("Usage: j4v --search <query> [--json]");
        let json_mode = args.iter().any(|a| a == "--json");
        let results = search(query).await;

        if json_mode {
            let items: Vec<serde_json::Value> = results.iter().map(|r| {
                serde_json::json!({"title": r.title, "url": r.url, "snippet": r.snippet})
            }).collect();
            let out = serde_json::json!({"query": query, "results": items});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            for (i, r) in results.iter().enumerate() {
                println!("{}. {}", i + 1, r.title);
                println!("   {}", r.url);
                if !r.snippet.is_empty() { println!("   {}", r.snippet); }
                println!();
            }
        }
        return Ok(());
    }

    // Interactive TUI mode
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<BgResult>();

    while app.running {
        terminal.draw(|f| ui(f, &app))?;

        // Check for background task results
        while let Ok(result) = rx.try_recv() {
            match result {
                BgResult::Search(results) => {
                    app.results = results;
                    app.selected = 0;
                    app.screen = Screen::Results;
                    app.loading = false;
                    app.status.clear();
                }
                BgResult::Page(title, lines) => {
                    app.page_title = title;
                    app.page_lines = lines;
                    app.loading = false;
                    app.status.clear();
                }
            }
        }

        if app.loading { app.spinner += 1; }

        if event::poll(std::time::Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_input(&mut app, key.code, &tx);
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if app.screen == Screen::Browse { app.page_scroll = app.page_scroll.saturating_sub(3); }
                    }
                    MouseEventKind::ScrollDown => {
                        if app.screen == Screen::Browse { app.page_scroll = app.page_scroll.saturating_add(3); }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(DisableMouseCapture)?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);
    match app.screen {
        Screen::Home => render_home(f, app, area),
        Screen::Results => render_results(f, app, area),
        Screen::Browse => render_browse(f, app, area),
    }
}

fn render_home(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Percentage(25), Constraint::Length(7),
        Constraint::Length(3), Constraint::Length(3), Constraint::Min(0),
    ]).split(area);

    let teal = Style::default().fg(Color::Rgb(0, 170, 170)).bold();
    let pink = Style::default().fg(Color::Rgb(255, 85, 255)).bold();
    let white = Style::default().fg(Color::Rgb(255, 255, 255)).bold();
    let logo = Paragraph::new(vec![
        Line::from(Span::styled(r"       __              __       __ __        _  ___              ", pink)),
        Line::from(Span::styled(r"      / /_  _______   / /_     / // /  _  __(_)/ _ )___  _____  ", pink)),
        Line::from(Span::styled(r"     / / / / / ___/  / __/    / // /_ | |/ / / __ \/ _ \/ ___/  ", white)),
        Line::from(Span::styled(r"  __/ / /_/ (__  )  / /_     /__ __/  | |/ / / /_/ /  __(__  )  ", white)),
        Line::from(Span::styled(r" /___/\__,_/____/   \__/       /_/    |___/_/_.___/\___/____/   ", teal)),
    ]).alignment(Alignment::Center);
    f.render_widget(logo, chunks[1]);

    let input_width = 50.min(area.width.saturating_sub(4));
    let input_area = Rect {
        x: area.x + (area.width.saturating_sub(input_width)) / 2,
        y: chunks[2].y, width: input_width, height: 3,
    };
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Search "));
    f.render_widget(input, input_area);
    f.set_cursor_position((input_area.x + 1 + app.cursor as u16, input_area.y + 1));

    let hint_text = if app.loading {
        format!("{} Searching...", app.spinner_char())
    } else {
        "Enter to search · Esc to quit".to_string()
    };
    let hint = Paragraph::new(Span::styled(
        hint_text, Style::default().fg(if app.loading { Color::Yellow } else { Color::DarkGray }),
    )).alignment(Alignment::Center);
    f.render_widget(hint, chunks[3]);
}

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), Constraint::Min(0), Constraint::Length(1),
    ]).split(area);

    let input_bar = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" j4v "));
    f.render_widget(input_bar, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    for (i, r) in app.results.iter().enumerate() {
        let style = if i == app.selected { Style::default().fg(Color::Cyan).bold() } else { Style::default().fg(Color::White).bold() };
        lines.push(Line::from(Span::styled(&r.title, style)));
        lines.push(Line::from(Span::styled(&r.url, Style::default().fg(Color::Green))));
        lines.push(Line::from(Span::styled(&r.snippet, Style::default().fg(Color::Gray))));
        lines.push(Line::from(""));
    }
    if app.results.is_empty() {
        lines.push(Line::from(Span::styled("No results.", Style::default().fg(Color::DarkGray))));
    }
    f.render_widget(Paragraph::new(lines), chunks[1]);

    let footer = Paragraph::new(Span::styled(
        " ↑↓ navigate · Enter open · / new search · Esc back", Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(footer, chunks[2]);
}

fn render_browse(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), Constraint::Min(0), Constraint::Length(1),
    ]).split(area);

    let title = if app.page_title.is_empty() { &app.page_url } else { &app.page_title };
    let url_bar = Paragraph::new(title.as_str())
        .block(Block::default().borders(Borders::ALL).title(format!(" {} ", &app.page_url)));
    f.render_widget(url_bar, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    if app.loading {
        lines.push(Line::from(Span::styled(
            format!("  {} Loading page...", app.spinner_char()),
            Style::default().fg(Color::Yellow),
        )));
    } else if !app.status.is_empty() {
        lines.push(Line::from(Span::styled(app.status.as_str(), Style::default().fg(Color::Yellow))));
    } else {
        let links = app.page_links();
        let selected_line = links.get(app.page_link_idx).map(|&(i, _)| i);
        for (i, pl) in app.page_lines.iter().enumerate() {
            match pl {
                PageLine::Heading(t) => lines.push(Line::from(Span::styled(t.as_str(), Style::default().fg(Color::Cyan).bold()))),
                PageLine::Text(t) => lines.push(Line::from(t.as_str())),
                PageLine::Link(text, _) => {
                    let is_selected = selected_line == Some(i);
                    let style = if is_selected {
                        Style::default().fg(Color::Rgb(0, 0, 0)).bg(Color::Rgb(0, 255, 255)).bold()
                    } else {
                        Style::default().fg(Color::Rgb(0, 255, 255)).underlined()
                    };
                    lines.push(Line::from(Span::styled(format!(" ▸ {}", text), style)));
                }
            }
        }
    }
    f.render_widget(Paragraph::new(lines).scroll((app.page_scroll, 0)), chunks[1]);

    let footer = Paragraph::new(Span::styled(
        " ↑↓ scroll · Tab/S-Tab links · Enter follow · Bksp back · h home · q quit", Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(footer, chunks[2]);
}

fn handle_input(app: &mut App, key: KeyCode, tx: &mpsc::UnboundedSender<BgResult>) {
    if app.loading { return; } // ignore input while loading
    match app.screen {
        Screen::Home => match key {
            KeyCode::Esc => app.running = false,
            KeyCode::Enter if !app.input.is_empty() => {
                app.loading = true;
                app.status = format!("{} Searching...", app.spinner_char());
                let query = app.input.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    let results = search(&query).await;
                    let _ = tx.send(BgResult::Search(results));
                });
            }
            KeyCode::Char(c) => { app.input.insert(app.cursor, c); app.cursor += 1; }
            KeyCode::Backspace if app.cursor > 0 => { app.cursor -= 1; app.input.remove(app.cursor); }
            KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
            KeyCode::Right => app.cursor = (app.cursor + 1).min(app.input.len()),
            _ => {}
        },
        Screen::Results => match key {
            KeyCode::Esc => app.screen = Screen::Home,
            KeyCode::Char('/') => { app.input.clear(); app.cursor = 0; app.screen = Screen::Home; }
            KeyCode::Up => app.selected = app.selected.saturating_sub(1),
            KeyCode::Down if app.selected + 1 < app.results.len() => app.selected += 1,
            KeyCode::Enter => {
                if let Some(r) = app.results.get(app.selected) {
                    let url = r.url.clone();
                    app.page_scroll = 0;
                    app.loading = true;
                    app.status = format!("{} Loading...", app.spinner_char());
                    app.screen = Screen::Browse;
                    app.page_lines.clear();
                    app.page_url = url.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let (title, lines) = browse(&url).await;
                        let _ = tx.send(BgResult::Page(title, lines));
                    });
                }
            }
            _ => {}
        },
        Screen::Browse => match key {
            KeyCode::Esc | KeyCode::Backspace => {
                if let Some(prev_url) = app.history.pop() {
                    // Go back
                    app.page_scroll = 0;
                    app.loading = true;
                    app.page_lines.clear();
                    app.page_url = prev_url.clone();
                    app.page_link_idx = 0;
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let (title, lines) = browse(&prev_url).await;
                        let _ = tx.send(BgResult::Page(title, lines));
                    });
                } else {
                    app.screen = Screen::Results;
                }
            }
            KeyCode::Char('h') => { app.screen = Screen::Home; app.history.clear(); }
            KeyCode::Char('q') => app.running = false,
            KeyCode::Up | KeyCode::Char('k') => app.page_scroll = app.page_scroll.saturating_sub(3),
            KeyCode::Down | KeyCode::Char('j') => app.page_scroll = app.page_scroll.saturating_add(3),
            KeyCode::PageUp => app.page_scroll = app.page_scroll.saturating_sub(20),
            KeyCode::PageDown => app.page_scroll = app.page_scroll.saturating_add(20),
            KeyCode::Tab => {
                let count = app.page_links().len();
                if count > 0 {
                    app.page_link_idx = (app.page_link_idx + 1) % count;
                    let links = app.page_links();
                    let line_idx = links[app.page_link_idx].0 as u16;
                    if line_idx < app.page_scroll || line_idx > app.page_scroll + 20 {
                        app.page_scroll = line_idx.saturating_sub(5);
                    }
                }
            }
            KeyCode::BackTab => {
                let count = app.page_links().len();
                if count > 0 {
                    app.page_link_idx = if app.page_link_idx == 0 { count - 1 } else { app.page_link_idx - 1 };
                    let links = app.page_links();
                    let line_idx = links[app.page_link_idx].0 as u16;
                    if line_idx < app.page_scroll || line_idx > app.page_scroll + 20 {
                        app.page_scroll = line_idx.saturating_sub(5);
                    }
                }
            }
            KeyCode::Enter => {
                let href = app.page_links().get(app.page_link_idx).map(|&(_, h)| h.to_string());
                if let Some(mut url) = href {
                    if url.starts_with('/') {
                        let base: String = app.page_url.split('/').take(3).collect::<Vec<_>>().join("/");
                        url = format!("{}{}", base, url);
                    }
                    if url.starts_with("http") {
                        app.history.push(app.page_url.clone());
                        app.page_scroll = 0;
                        app.page_link_idx = 0;
                        app.loading = true;
                        app.page_lines.clear();
                        app.page_url = url.clone();
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let (title, lines) = browse(&url).await;
                            let _ = tx.send(BgResult::Page(title, lines));
                        });
                    }
                }
            }
            _ => {}
        },
    }
}

async fn search(query: &str) -> Vec<SearchResult> {
    let url = format!("https://html.duckduckgo.com/html/?q={}", query);
    let client = reqwest::Client::new();
    let resp = client.get(&url).header("User-Agent", "Mozilla/5.0").send().await;
    let Ok(resp) = resp else { return vec![] };
    let Ok(body) = resp.text().await else { return vec![] };

    let mut results = Vec::new();
    for chunk in body.split("class=\"result__a\"").skip(1).take(10) {
        let title = extract_between(chunk, ">", "</a>").unwrap_or_default();
        let raw_url = extract_between(chunk, "href=\"", "\"").unwrap_or_default();
        let snippet = chunk.split("class=\"result__snippet\"")
            .nth(1).and_then(|s| extract_between(s, ">", "</")).unwrap_or_default();

        // Extract real URL from DDG redirect
        let url = if raw_url.contains("uddg=") {
            extract_between(raw_url, "uddg=", "&")
                .or_else(|| raw_url.split("uddg=").nth(1))
                .unwrap_or(raw_url)
        } else {
            raw_url
        };
        let url = url_decode(url);

        if !title.is_empty() {
            results.push(SearchResult {
                title: html_decode(title), url, snippet: html_decode(snippet),
            });
        }
    }
    results
}

async fn browse(url: &str) -> (String, Vec<PageLine>) {
    let client = reqwest::Client::new();

    // Try Cloudflare markdown-for-agents first (site must opt-in)
    let body = if let Ok(resp) = client.get(url)
        .header("Accept", "text/markdown")
        .header("User-Agent", "j4v/0.1")
        .send().await
    {
        if resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("markdown"))
            .unwrap_or(false)
        {
            resp.text().await.ok()
        } else {
            None
        }
    } else {
        None
    };

    // Fall back to Jina Reader
    let body = match body {
        Some(md) => md,
        None => {
            let jina_url = format!("https://r.jina.ai/{}", url);
            match client.get(&jina_url)
                .header("User-Agent", "j4v/0.1")
                .send().await
            {
                Ok(resp) => resp.text().await.unwrap_or_else(|_| "Failed to read.".into()),
                Err(_) => return (String::new(), vec![PageLine::Text("Failed to load.".into())]),
            }
        }
    };

    // Parse markdown response
    let mut title = String::new();
    let mut lines: Vec<PageLine> = Vec::new();

    for line in body.lines() {
        if line.starts_with("Title: ") && title.is_empty() {
            title = line[7..].to_string();
        } else if line.starts_with("# ") {
            lines.push(PageLine::Heading(line[2..].to_string()));
        } else if line.starts_with("## ") {
            lines.push(PageLine::Heading(line[3..].to_string()));
        } else if line.starts_with("### ") {
            lines.push(PageLine::Heading(line[4..].to_string()));
        } else if line.starts_with("* ") || line.starts_with("- ") {
            lines.push(PageLine::Text(format!("  • {}", &line[2..])));
        } else if line.starts_with("[") && line.contains("](") {
            // [text](url)
            if let (Some(text_end), Some(url_start)) = (line.find("]("), line.rfind(')')) {
                let text = &line[1..text_end];
                let href = &line[text_end+2..url_start];
                lines.push(PageLine::Link(text.to_string(), href.to_string()));
            } else {
                lines.push(PageLine::Text(line.to_string()));
            }
        } else if line.starts_with("*   [") || line.starts_with("-   [") {
            // list item with link
            let inner = &line[4..];
            if let (Some(text_end), Some(url_start)) = (inner.find("]("), inner.rfind(')')) {
                let text = &inner[1..text_end];
                let href = &inner[text_end+2..url_start];
                lines.push(PageLine::Link(format!("  • {}", text), href.to_string()));
            } else {
                lines.push(PageLine::Text(format!("  • {}", &line[4..])));
            }
        } else if line.starts_with("URL Source:") || line.starts_with("Published Time:") || line.starts_with("Warning:") || line.starts_with("Markdown Content:") {
            // Skip Jina metadata
        } else if !line.is_empty() {
            lines.push(PageLine::Text(line.to_string()));
        }
    }

    if lines.is_empty() {
        lines.push(PageLine::Text("No content extracted.".into()));
    }

    (title, lines)
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    let j = s[i..].find(end)? + i;
    Some(&s[i..j])
}

fn strip_tags(s: &str) -> String {
    let mut r = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c { '<' => in_tag = true, '>' => in_tag = false, _ if !in_tag => r.push(c), _ => {} }
    }
    r
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#x27;", "'").replace("&#39;", "'")
        .replace("&nbsp;", " ").replace("<b>", "").replace("</b>", "")
}

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            let hex = [hi, lo];
            if let Ok(s) = std::str::from_utf8(&hex) {
                if let Ok(val) = u8::from_str_radix(s, 16) {
                    result.push(val as char);
                    continue;
                }
            }
            result.push('%');
            result.push(hi as char);
            result.push(lo as char);
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}
