use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap, Scrollbar, ScrollbarState, ScrollbarOrientation},
    Terminal,
};
use std::io;
use std::time::Duration;
use anyhow::Result;
use serde_json::Value;

// --- Problem Detail View ---

struct ProblemAppState {
    scroll_description: u16,
    scroll_tests: u16,
    // 0: Description, 1: Public Tests
    focus_index: usize,
    max_scroll_description: u16,
    max_scroll_tests: u16,
}

impl ProblemAppState {
    fn new() -> Self {
        Self {
            scroll_description: 0,
            scroll_tests: 0,
            focus_index: 0,
            max_scroll_description: 0,
            max_scroll_tests: 0,
        }
    }

    fn next_focus(&mut self) {
        self.focus_index = (self.focus_index + 1) % 2;
    }

    fn scroll_up(&mut self) {
        if self.focus_index == 0 {
            if self.scroll_description > 0 {
                self.scroll_description -= 1;
            }
        } else {
             if self.scroll_tests > 0 {
                self.scroll_tests -= 1;
            }
        }
    }

    fn scroll_down(&mut self) {
        if self.focus_index == 0 {
             // We don't strictly cap strictly at max_scroll because measuring height exactly is tricky before render,
             // but we can try to be reasonable or just let it scroll.
             // Ratatui's paragraph scroll is just an offset.
             self.scroll_description += 1;
        } else {
             self.scroll_tests += 1;
        }
    }
}

pub fn draw_problem(problem: &Value) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_problem_app(&mut terminal, problem);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_problem_app<B: Backend>(terminal: &mut Terminal<B>, problem: &Value) -> io::Result<()> {
    let mut state = ProblemAppState::new();

    loop {
        terminal.draw(|f| ui_problem(f, problem, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => state.next_focus(),
                    KeyCode::Char('j') | KeyCode::Down => state.scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => state.scroll_up(),
                    _ => {}
                }
            }
        }
    }
}

fn ui_problem(f: &mut ratatui::Frame, problem: &Value, state: &mut ProblemAppState) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Length(3), // General Status
                Constraint::Min(5),    // Description (Scrollable)
                Constraint::Length(5), // Public Tests (Scrollable)
                Constraint::Length(3), // User Status
                Constraint::Length(1), // Help footer
            ]
            .as_ref(),
        )
        .split(size);

    // 1. Title
    let title = problem["title"].as_str().unwrap_or("Unknown Title");
    let title_paragraph = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Title"));
    f.render_widget(title_paragraph, chunks[0]);

    // 2. General Status
    let time_limit = problem["timeLimit"].as_i64().unwrap_or(0);
    let memory_limit = problem["memoryLimit"].as_i64().unwrap_or(0);
    let full_score = problem["fullScore"].as_i64().unwrap_or(0);

    let status_text = format!(
        "Time Limit: {}ms | Memory Limit: {}MB | Full Score: {}",
        time_limit, memory_limit, full_score
    );
    let status_paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("General Status"));
    f.render_widget(status_paragraph, chunks[1]);

    // 3. Description
    let description = problem["description"].as_str().unwrap_or("No description provided.");
    let desc_style = if state.focus_index == 0 {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD) // Highlight title border? Or content?
    } else {
        Style::default()
    };
    let desc_border_style = if state.focus_index == 0 {
         Style::default().fg(Color::Green)
    } else {
         Style::default()
    };

    // We need to calculate line count for scrollbar if possible, but Textwrap is done by Paragraph.
    // We can just estimate or use state.

    let description_paragraph = Paragraph::new(description)
        .wrap(Wrap { trim: true })
        .style(desc_style)
        .scroll((state.scroll_description, 0))
        .block(Block::default().borders(Borders::ALL).title("Description").border_style(desc_border_style));
    f.render_widget(description_paragraph, chunks[2]);

    // Scrollbar for Description
    if state.focus_index == 0 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(description.lines().count()) // Approximate
            .position(state.scroll_description as usize);
        f.render_stateful_widget(
            scrollbar,
            chunks[2].inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }), // Simple margin
            &mut scrollbar_state,
        );
    }

    // 4. Public Tests
    let sample_testcases = problem["sampleTestcases"].as_array();
    let tests_text = if let Some(tests) = sample_testcases {
        if tests.is_empty() {
             "No public tests available.".to_string()
        } else {
             tests.iter().enumerate().map(|(i, t)| {
                 let input = t["input"].as_str().unwrap_or("").trim();
                 let output = t["output"].as_str().unwrap_or("").trim();
                 format!("Test #{}:\nInput: {}\nOutput: {}", i + 1, input, output)
             }).collect::<Vec<_>>().join("\n---\n")
        }
    } else {
        "No public tests available.".to_string()
    };

    let tests_border_style = if state.focus_index == 1 {
         Style::default().fg(Color::Green)
    } else {
         Style::default()
    };

    let tests_paragraph = Paragraph::new(tests_text.clone())
        .wrap(Wrap { trim: true })
        .scroll((state.scroll_tests, 0))
        .block(Block::default().borders(Borders::ALL).title("Public Tests").border_style(tests_border_style));
    f.render_widget(tests_paragraph, chunks[3]);

    // Scrollbar for Tests
    if state.focus_index == 1 {
        let scrollbar = Scrollbar::default()
             .orientation(ScrollbarOrientation::VerticalRight)
             .begin_symbol(Some("↑"))
             .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::default()
             .content_length(tests_text.lines().count())
             .position(state.scroll_tests as usize);
        f.render_stateful_widget(
            scrollbar,
            chunks[3].inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }

    // 5. User Status
    let user_detail = &problem["userDetail"];
    let attempted = user_detail["attempted"].as_i64().unwrap_or(0);
    let solved = user_detail["solved"].as_i64().unwrap_or(0);

    let user_status_text = format!(
        "Global Stats - Attempted: {} | Solved: {}",
        attempted, solved
    );
    let user_status_paragraph = Paragraph::new(user_status_text)
        .block(Block::default().borders(Borders::ALL).title("User Status"));
    f.render_widget(user_status_paragraph, chunks[4]);

    // Footer
    let footer = Paragraph::new("Press <Tab> to switch focus, <Up/Down/j/k> to scroll, <q> to quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[5]);
}

// --- Table View (Generic) ---

pub struct TableData {
    pub title: String,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn draw_table(data: TableData) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_table_app(&mut terminal, data);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_table_app<B: Backend>(terminal: &mut Terminal<B>, data: TableData) -> io::Result<()> {
    let mut state = TableState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| ui_table(f, &data, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i >= data.rows.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    data.rows.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui_table(f: &mut ratatui::Frame, data: &TableData, state: &mut TableState) {
    let size = f.area();
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(1)
        .split(size);

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::Blue);

    let header_cells = data.header.iter()
        .map(|h| Cell::from(h.as_str()).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);

    let rows = data.rows.iter().map(|item| {
        let cells = item.iter().map(|c| Cell::from(c.as_str()));
        Row::new(cells).height(1).bottom_margin(1)
    });

    let col_count = data.header.len();
    let constraints: Vec<Constraint> = (0..col_count).map(|_| Constraint::Ratio(1, col_count as u32)).collect();

    let t = Table::new(rows, constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(data.title.as_str()))
        .row_highlight_style(selected_style)
        .highlight_symbol(">> ");

    f.render_stateful_widget(t, rects[0], state);
}
