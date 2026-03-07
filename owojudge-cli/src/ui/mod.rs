use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

pub mod announcement;
pub mod contest;
pub mod problem;
pub mod table;

pub use announcement::draw_announcement;
pub use contest::draw_contest;
pub use problem::draw_problem;
pub use table::{draw_table, TableData};

/// Shared scroll offset used by single-pane viewer UIs.
pub(crate) struct ScrollState {
    pub offset: u16,
}

impl ScrollState {
    pub fn new() -> Self {
        Self { offset: 0 }
    }
}

/// Restores raw mode, alternate screen, and cursor visibility when dropped.
/// Ensures cleanup even if a panic occurs inside `with_terminal`.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

/// Manages terminal setup and teardown around a TUI closure.
pub(crate) fn with_terminal<F>(f: F) -> Result<()>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> Result<()>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _guard = TerminalGuard;
    f(&mut terminal)
    // _guard drops here, restoring terminal state (including on panic)
}
