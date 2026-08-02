use std::io;
use std::time::Duration;

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
    NewName(String),
}

/// `title`/`header` are the popup border title and the instructions line above the
/// list. `allow_new` controls whether a "New name..." row is offered at the top of
/// the list (naming can create a new binding; goto can only jump to an existing one).
pub struct PickerOptions<'a> {
    pub title: &'a str,
    pub header: &'a str,
    pub allow_new: bool,
}

pub fn run(note_names: &[String], options: &PickerOptions) -> io::Result<Option<Selection>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_picker_loop(&mut terminal, note_names, options);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Highest selectable list index: the filtered notes, plus one more for the
/// "New name..." row when `allow_new` is set. Saturates to 0 when the list is empty.
fn max_selectable_index(filtered_count: usize, allow_new: bool) -> usize {
    let extra = if allow_new { 1 } else { 0 };
    (filtered_count + extra).saturating_sub(1)
}

fn run_picker_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    note_names: &[String],
    options: &PickerOptions,
) -> io::Result<Option<Selection>> {
    let mut state = PickerState::default();

    loop {
        let filtered = filtered_names(note_names, &state.filter);
        let max_index = max_selectable_index(filtered.len(), options.allow_new);
        state.clamp_selection(max_index);

        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Clear, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(4),
                    Constraint::Length(3),
                    Constraint::Length(2),
                ])
                .split(area);

            let header = Paragraph::new(options.header).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", options.title)),
            );
            frame.render_widget(header, chunks[0]);

            let mut items = Vec::new();
            if options.allow_new {
                items.push(ListItem::new(Line::from(new_name_label(&state.filter))));
            }
            items.extend(
                filtered
                    .iter()
                    .map(|name| ListItem::new(Line::from(name.as_str()))),
            );
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Options "))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            let mut list_state = ListState::default();
            list_state.select(Some(state.selected));
            frame.render_stateful_widget(list, chunks[1], &mut list_state);

            let filter = Paragraph::new(state.filter.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Filter "));
            frame.render_widget(filter, chunks[2]);
            frame.set_cursor_position((
                chunks[2].x + 1 + state.filter.len() as u16,
                chunks[2].y + 1,
            ));

            let help = Paragraph::new(
                "Type: filter  Enter: select  Up/Down: move  Backspace: delete  Esc: cancel",
            );
            frame.render_widget(help, chunks[3]);
        })?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Up => {
                    state.selected = state.selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    state.selected = (state.selected + 1).min(max_index);
                }
                KeyCode::Enter => {
                    if options.allow_new && state.selected == 0 {
                        return Ok(Some(Selection::NewName(state.filter.clone())));
                    }
                    let idx = if options.allow_new {
                        state.selected - 1
                    } else {
                        state.selected
                    };
                    if let Some(name) = filtered.get(idx) {
                        return Ok(Some(Selection::Existing(name.clone())));
                    }
                }
                KeyCode::Backspace => {
                    state.filter.pop();
                    state.selected = 0;
                }
                KeyCode::Char(c) if accepts_char(key.modifiers) => {
                    state.filter.push(c);
                    state.selected = 0;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[derive(Default)]
struct PickerState {
    filter: String,
    selected: usize,
}

impl PickerState {
    fn clamp_selection(&mut self, filtered_count: usize) {
        self.selected = self.selected.min(filtered_count);
    }
}

fn filtered_names(note_names: &[String], filter: &str) -> Vec<String> {
    let needle = filter.trim().to_ascii_lowercase();
    note_names
        .iter()
        .filter(|name| needle.is_empty() || name.to_ascii_lowercase().contains(&needle))
        .cloned()
        .collect()
}

fn new_name_label(filter: &str) -> String {
    let trimmed = filter.trim();
    if trimmed.is_empty() {
        "New name...".to_string()
    } else {
        format!("New name: {}", trimmed)
    }
}

fn accepts_char(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_names_matches_case_insensitively() {
        let note_names = vec![
            "Alpha".to_string(),
            "beta project".to_string(),
            "Gamma".to_string(),
        ];

        assert_eq!(
            filtered_names(&note_names, "PROJ"),
            vec!["beta project".to_string()]
        );
    }

    #[test]
    fn filtered_names_returns_all_for_blank_filter() {
        let note_names = vec!["alpha".to_string(), "beta".to_string()];

        assert_eq!(filtered_names(&note_names, "  "), note_names);
    }

    #[test]
    fn new_name_label_uses_trimmed_filter() {
        assert_eq!(new_name_label(""), "New name...");
        assert_eq!(new_name_label("  next note  "), "New name: next note");
    }

    #[test]
    fn selection_clamps_to_new_name_when_filter_shrinks_list() {
        let mut state = PickerState {
            filter: "alpha".to_string(),
            selected: 3,
        };

        state.clamp_selection(1);

        assert_eq!(state.selected, 1);
        state.clamp_selection(0);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn max_selectable_index_with_new_row_includes_the_extra_slot() {
        assert_eq!(max_selectable_index(2, true), 2);
        assert_eq!(max_selectable_index(0, true), 0);
    }

    #[test]
    fn max_selectable_index_without_new_row_indexes_filtered_directly() {
        assert_eq!(max_selectable_index(2, false), 1);
        assert_eq!(max_selectable_index(0, false), 0);
    }
}
