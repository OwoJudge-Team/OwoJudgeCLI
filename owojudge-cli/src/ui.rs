use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;
use std::time::Duration;
use anyhow::Result;
use serde_json::Value;

pub fn draw_problem(problem: &Value) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, problem);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, problem: &Value) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, problem))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, problem: &Value) {
    let size = f.area();

    // Layout
    // 1. Title
    // 2. General Status
    // 3. Description (takes most space)
    // 4. Public Tests
    // 5. User Status

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Length(3), // General Status
                Constraint::Min(5),    // Description
                Constraint::Length(5), // Public Tests (Sample) - Adjustable?
                Constraint::Length(3), // User Status
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
    // Simple text handling for now.
    // Ideally we would parse markdown but standard textwrap is a good start.
    let description_paragraph = Paragraph::new(description)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Description"));
    f.render_widget(description_paragraph, chunks[2]);

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

    let tests_paragraph = Paragraph::new(tests_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Public Tests"));
    f.render_widget(tests_paragraph, chunks[3]);

    // 5. User Status
    let user_detail = &problem["userDetail"];
    let attempted = user_detail["attempted"].as_i64().unwrap_or(0);
    let solved = user_detail["solved"].as_i64().unwrap_or(0);
    // Determine if current user solved it? The API response doesn't seem to have "isSolved" directly on root,
    // but userDetail might be aggregate stats or specific user stats.
    // Based on API docs "userDetail": { "solved": 0, "attempted": 0 } seems to be aggregate.
    // Wait, the API docs say:
    // "userDetail": { "solved": 0, "attempted": 0 }
    // It usually means "Global stats".
    // But the requirements say "User status (attempted, solved, unattempted)".
    // If this CLI is for a specific user, maybe we can deduce from somewhere?
    // Actually, looking at the problem response in the issue description:
    // "userDetail": { "attempted": 849, "solved": 563 }
    // These look like global stats.
    // "unattempted" for a specific user is 1 if attempted=0.
    // Let's display the stats provided.

    let user_status_text = format!(
        "Global Stats - Attempted: {} | Solved: {}",
        attempted, solved
    );
    let user_status_paragraph = Paragraph::new(user_status_text)
        .block(Block::default().borders(Borders::ALL).title("User Status"));
    f.render_widget(user_status_paragraph, chunks[4]);
}
