use anyhow::Result;
use crate::models::Announcement;
use crate::ui::{with_terminal, ScrollState};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::time::Duration;

pub fn draw_announcement(announcement: &Announcement, plain: bool) -> Result<()> {
    if plain {
        println!("Topic: {}", announcement.topic);
        println!("Date:  {}", announcement.timestamp);
        println!("\n{}", announcement.content);
        return Ok(());
    }
    with_terminal(|terminal| run_announcement_app(terminal, announcement))
}

fn run_announcement_app<B: Backend>(terminal: &mut Terminal<B>, announcement: &Announcement) -> Result<()> {
    let mut state = ScrollState::new();
    let content_lines = announcement.content.lines().count() as u16;

    loop {
        // Recompute the max scroll offset each frame based on current terminal height.
        // Fixed rows: topic(3) + timestamp(3) + footer(1) = 7; content pane takes the rest.
        let term_height = terminal.size()?.height;
        let visible = term_height.saturating_sub(7);
        let max_scroll = content_lines.saturating_sub(visible);

        terminal.draw(|f| ui_announcement(f, announcement, &state))?;

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

fn ui_announcement(
    f: &mut ratatui::Frame,
    announcement: &Announcement,
    state: &ScrollState,
) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Topic
            Constraint::Length(3), // Timestamp
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Footer
        ])
        .split(size);

    let topic_p = Paragraph::new(announcement.topic.as_str())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Topic"));
    f.render_widget(topic_p, chunks[0]);

    let time_p = Paragraph::new(announcement.timestamp.as_str())
        .block(Block::default().borders(Borders::ALL).title("Date"));
    f.render_widget(time_p, chunks[1]);

    let content_p = Paragraph::new(announcement.content.as_str())
        .wrap(Wrap { trim: true })
        .scroll((state.offset, 0))
        .block(Block::default().borders(Borders::ALL).title("Content"));
    f.render_widget(content_p, chunks[2]);

    let footer = Paragraph::new("Press <Up/Down/j/k> to scroll, <q> to quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
