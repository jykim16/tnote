use std::io;
use std::time::Duration;

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

pub enum Selection {
    Existing(String),
    PromptNew,
}

pub fn run(note_names: &[String]) -> io::Result<Option<Selection>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_picker_loop(&mut terminal, note_names);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_picker_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    note_names: &[String],
) -> io::Result<Option<Selection>> {
    let mut selected = 0usize;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Clear, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(2),
                ])
                .split(area);

            let header = Paragraph::new("Select an existing named note, or choose New name... to enter one in tmux.")
                .block(Block::default().borders(Borders::ALL).title(" tnote name "));
            frame.render_widget(header, chunks[0]);

            let mut items = vec![ListItem::new(Line::from("New name..."))];
            items.extend(note_names.iter().map(|name| ListItem::new(Line::from(name.as_str()))));
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Options "))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            let mut state = ListState::default();
            state.select(Some(selected.min(note_names.len())));
            frame.render_stateful_widget(list, chunks[1], &mut state);

            let help = Paragraph::new("Enter: confirm  Up/Down: move  Esc: cancel");
            frame.render_widget(help, chunks[2]);
        })?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    selected = (selected + 1).min(note_names.len());
                }
                KeyCode::Enter => {
                    if selected == 0 {
                        return Ok(Some(Selection::PromptNew));
                    }
                    if let Some(name) = note_names.get(selected - 1) {
                        return Ok(Some(Selection::Existing(name.clone())));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}
