use anyhow::Result;
use crate::models::Contest;
use crate::ui::{with_terminal, ScrollState};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::time::Duration;

pub fn draw_contest(contest: &Contest, plain: bool) -> Result<()> {
    if plain {
        println!("Contest: {}", contest.title);
        println!("Start: {}", contest.start_time);
        println!("End:   {}", contest.end_time.as_deref().unwrap_or("Open-ended"));
        println!("Submissions Close: {}", contest.submission_end_time);
        println!("GM Allowed: {}", contest.can_apply_gm);
        println!("\nDescription:\n{}", contest.description);
        println!("\nProblems:");
        for p in &contest.problems {
            println!("  - Problem SN {}: ({} points)", p.serial_number, p.score);
        }
        return Ok(());
    }
    with_terminal(|terminal| run_contest_app(terminal, contest))
}

fn run_contest_app<B: Backend>(terminal: &mut Terminal<B>, contest: &Contest) -> Result<()> {
    let mut state = ScrollState::new();

    let mut content_text = format!("Description:\n{}\n\nProblems:\n", contest.description);
    for p in &contest.problems {
        content_text.push_str(&format!(
            "- Problem SN {}: ({} points)\n",
            p.serial_number, p.score
        ));
    }
    let content_lines = content_text.lines().count() as u16;

    loop {
        // Recompute the max scroll offset each frame based on current terminal height.
        // Fixed rows: title(3) + info(5) + footer(1) = 9; content pane takes the rest.
        let term_height = terminal.size()?.height;
        let visible = term_height.saturating_sub(9);
        let max_scroll = content_lines.saturating_sub(visible);

        terminal.draw(|f| ui_contest(f, contest, &state, &content_text))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => {
                        if state.offset < max_scroll {
                            state.offset += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.offset = state.offset.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui_contest(
    f: &mut ratatui::Frame,
    contest: &Contest,
    state: &ScrollState,
    content_text: &str,
) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Times & Info
            Constraint::Min(5),    // Description & Problems
            Constraint::Length(1), // Footer
        ])
        .split(size);

    let title_p = Paragraph::new(contest.title.as_str())
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Contest"));
    f.render_widget(title_p, chunks[0]);

    let end_str = contest.end_time.as_deref().unwrap_or("Open-ended");
    let info_text = format!(
        "Start: {}\nEnd:   {}\nSubmissions Close: {}\nGM Allowed: {}",
        contest.start_time, end_str, contest.submission_end_time, contest.can_apply_gm
    );
    let info_p = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Information"));
    f.render_widget(info_p, chunks[1]);

    let content_p = Paragraph::new(content_text)
        .wrap(Wrap { trim: true })
        .scroll((state.offset, 0))
        .block(Block::default().borders(Borders::ALL).title("Content"));
    f.render_widget(content_p, chunks[2]);

    let footer = Paragraph::new("Press <Up/Down/j/k> to scroll, <q> to quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
