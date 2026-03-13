use anyhow::Result;
use crate::ui::with_terminal;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Terminal,
};
use std::time::Duration;

pub struct TableData {
    pub title: String,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn draw_table(data: TableData, plain: bool) -> Result<()> {
    if plain {
        println!("{}", data.header.join("\t"));
        for row in &data.rows {
            println!("{}", row.join("\t"));
        }
        return Ok(());
    }
    with_terminal(|terminal| run_table_app(terminal, data))
}

fn run_table_app<B: Backend>(terminal: &mut Terminal<B>, data: TableData) -> Result<()> {
    let mut state = TableState::default();
    if !data.rows.is_empty() {
        state.select(Some(0));
    }

    loop {
        terminal.draw(|f| ui_table(f, &data, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !data.rows.is_empty() {
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
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !data.rows.is_empty() {
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

    let header_cells = data
        .header
        .iter()
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
    if col_count == 0 {
        return;
    }
    let constraints: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Ratio(1, col_count as u32))
        .collect();

    let t = Table::new(rows, constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(data.title.as_str()),
        )
        .row_highlight_style(selected_style)
        .highlight_symbol(">> ");

    f.render_stateful_widget(t, rects[0], state);
}
