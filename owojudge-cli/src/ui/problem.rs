use anyhow::Result;
use crate::models::{Problem, SampleTestcase};
use crate::ui::with_terminal;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Terminal,
};
use std::time::Duration;

struct ProblemAppState {
    scroll_description: u16,
    scroll_tests: u16,
    // 0: Description, 1: Public Tests
    focus_index: usize,
}

impl ProblemAppState {
    fn new() -> Self {
        Self {
            scroll_description: 0,
            scroll_tests: 0,
            focus_index: 0,
        }
    }

    fn next_focus(&mut self) {
        self.focus_index = (self.focus_index + 1) % 2;
    }

    fn scroll_up(&mut self) {
        if self.focus_index == 0 {
            self.scroll_description = self.scroll_description.saturating_sub(1);
        } else {
            self.scroll_tests = self.scroll_tests.saturating_sub(1);
        }
    }

    fn scroll_down(&mut self, max_desc: u16, max_tests: u16) {
        if self.focus_index == 0 && self.scroll_description < max_desc {
            self.scroll_description += 1;
        } else if self.focus_index != 0 && self.scroll_tests < max_tests {
            self.scroll_tests += 1;
        }
    }
}

pub fn draw_problem(problem: &Problem, plain: bool) -> Result<()> {
    if plain {
        println!("Title: {}", problem.title);
        println!(
            "Time Limit: {}ms | Memory Limit: {}MB | Full Score: {}",
            problem.time_limit, problem.memory_limit, problem.full_score
        );
        if let Some(desc) = &problem.description {
            println!("\nDescription:\n{}", desc);
        }
        println!("\nPublic Tests:\n{}", format_tests(problem.sample_testcases.as_ref()));
        if let Some(ud) = &problem.user_detail {
            println!("\nGlobal Stats - Attempted: {} | Solved: {}", ud.attempted, ud.solved);
        }
        return Ok(());
    }
    with_terminal(|terminal| run_problem_app(terminal, problem))
}

fn run_problem_app<B: Backend>(terminal: &mut Terminal<B>, problem: &Problem) -> Result<()> {
    let mut state = ProblemAppState::new();

    // Content is immutable for the lifetime of the viewer; compute line counts once.
    let description = problem.description.as_deref().unwrap_or("");
    let tests_text = format_tests(problem.sample_testcases.as_ref());
    let desc_lines = description.lines().count() as u16;
    let tests_lines = tests_text.lines().count() as u16;

    loop {
        // Recompute max scroll each frame using current terminal height.
        // Fixed rows: title(3) + status(3) + tests(5) + user_status(3) + footer(1) = 15.
        // The description pane gets the remaining rows; subtract 2 for its own borders.
        let term_height = terminal.size()?.height;
        let desc_visible = term_height.saturating_sub(15 + 2);
        let max_desc = desc_lines.saturating_sub(desc_visible);
        // Tests pane is fixed at 5 rows; subtract 2 for borders = 3 visible lines.
        let max_tests = tests_lines.saturating_sub(3);

        terminal.draw(|f| ui_problem(f, problem, &state, description, &tests_text))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => state.next_focus(),
                    KeyCode::Char('j') | KeyCode::Down => state.scroll_down(max_desc, max_tests),
                    KeyCode::Char('k') | KeyCode::Up => state.scroll_up(),
                    _ => {}
                }
            }
        }
    }
}

fn format_tests(sample_testcases: Option<&Vec<SampleTestcase>>) -> String {
    if let Some(tests) = sample_testcases {
        if tests.is_empty() {
            "No public tests available.".to_string()
        } else {
            tests
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let input = t.input.trim();
                    let output = t.output.trim();
                    format!("Test #{}:\nInput: {}\nOutput: {}", i + 1, input, output)
                })
                .collect::<Vec<_>>()
                .join("\n---\n")
        }
    } else {
        "No public tests available.".to_string()
    }
}

fn ui_problem(
    f: &mut ratatui::Frame,
    problem: &Problem,
    state: &ProblemAppState,
    description: &str,
    tests_text: &str,
) {
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
    let title_paragraph = Paragraph::new(problem.title.as_str())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Title"));
    f.render_widget(title_paragraph, chunks[0]);

    // 2. General Status
    let status_text = format!(
        "Time Limit: {}ms | Memory Limit: {}MB | Full Score: {}",
        problem.time_limit, problem.memory_limit, problem.full_score
    );
    let status_paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("General Status"));
    f.render_widget(status_paragraph, chunks[1]);

    // 3. Description
    let desc_border_style = if state.focus_index == 0 {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let description_paragraph = Paragraph::new(description)
        .wrap(Wrap { trim: true })
        .scroll((state.scroll_description, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Description")
                .border_style(desc_border_style),
        );
    f.render_widget(description_paragraph, chunks[2]);

    // Scrollbar for Description
    if state.focus_index == 0 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(description.lines().count())
            .position(state.scroll_description as usize);
        f.render_stateful_widget(
            scrollbar,
            chunks[2].inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // 4. Public Tests
    let tests_border_style = if state.focus_index == 1 {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let tests_paragraph = Paragraph::new(tests_text)
        .wrap(Wrap { trim: true })
        .scroll((state.scroll_tests, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Public Tests")
                .border_style(tests_border_style),
        );
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
            chunks[3].inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // 5. User Status
    let user_status_text = if let Some(ud) = &problem.user_detail {
        format!("Global Stats - Attempted: {} | Solved: {}", ud.attempted, ud.solved)
    } else {
        "Global Stats - No data available".to_string()
    };
    let user_status_paragraph = Paragraph::new(user_status_text)
        .block(Block::default().borders(Borders::ALL).title("User Status"));
    f.render_widget(user_status_paragraph, chunks[4]);

    // Footer
    let footer = Paragraph::new("Press <Tab> to switch focus, <Up/Down/j/k> to scroll, <q> to quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[5]);
}
