use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};
use ratatui::{Terminal, TerminalOptions, Viewport};

/// Parse a dimension string: "75%" → percentage of `total`; "80" → literal columns/lines.
fn resolve_dim(s: &str, total: u16) -> u16 {
    if let Some(pct) = s.strip_suffix('%') {
        let p: u16 = pct.trim().parse().unwrap_or(100);
        ((total as u32 * p as u32) / 100).min(total as u32) as u16
    } else {
        s.trim().parse::<u16>().unwrap_or(total).min(total)
    }
}

fn make_rect(width: &str, height: &str) -> io::Result<(Rect, u16, u16)> {
    let (term_w, term_h) = terminal::size()?;
    let popup_w = resolve_dim(width, term_w);
    let popup_h = resolve_dim(height, term_h);
    let rect = Rect::new(term_w.saturating_sub(popup_w), 0, popup_w, popup_h);
    let inner_w = popup_w.saturating_sub(2);
    let inner_h = popup_h.saturating_sub(2);
    Ok((rect, inner_w, inner_h))
}

pub fn run(file: &Path, label: &str, width: &str, height: &str) -> io::Result<()> {
    // Initial sizing for the PTY (pre-alternate-screen).
    let (_, init_iw, init_ih) = make_rect(width, height)?;

    let background = capture_pane_snapshot();

    // ── PTY ────────────────────────────────────────────────────────────────────

    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: init_ih,
            cols: init_iw,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| io::Error::other(e.to_string()))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let mut cmd = CommandBuilder::new(&editor);
    cmd.arg(file);
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| io::Error::other(e.to_string()))?;
    drop(pair.slave);

    let mut pty_writer = pair
        .master
        .take_writer()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let mut pty_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| io::Error::other(e.to_string()))?;

    // Read PTY output on a background thread; send bytes to main thread.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match pty_reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut parser = vt100::Parser::new(init_ih, init_iw, 0);

    // ── Terminal setup ─────────────────────────────────────────────────────────

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    // Re-query size after entering alternate screen — some terminals (e.g. iTerm
    // fullscreen) report a different size at this point.
    let (mut rect, mut inner_w, mut inner_h) = make_rect(width, height)?;

    if let Some(bg) = background {
        paint_background(&bg)?;
    }

    // Pre-clear the popup rect.
    execute!(io::stdout(), ratatui::crossterm::style::ResetColor)?;
    let blank = " ".repeat(rect.width as usize);
    for row in rect.y..rect.y + rect.height {
        execute!(io::stdout(), cursor::MoveTo(rect.x, row))?;
        write!(io::stdout(), "{}", blank)?;
    }
    io::stdout().flush()?;

    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Fixed(rect),
        },
    )?;

    // ── Event loop ─────────────────────────────────────────────────────────────

    'main: loop {
        // Drain all pending PTY output; exit if the child process closed the PTY.
        loop {
            match rx.try_recv() {
                Ok(data) => parser.process(&data),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break 'main,
            }
        }

        terminal.draw(|frame| {
            // Border
            let block = Block::default()
                .borders(Borders::ALL)
                .title_top(Line::from(format!(" tnote: {} ", label)));
            frame.render_widget(block, rect);

            // PTY screen cells
            let screen = parser.screen();
            {
                let buf = frame.buffer_mut();
                for row in 0..inner_h {
                    for col in 0..inner_w {
                        if let Some(cell) = screen.cell(row, col) {
                            let s = cell.contents();
                            let s = if s.is_empty() { " ".to_string() } else { s };
                            buf.set_string(rect.x + 1 + col, rect.y + 1 + row, s, cell_style(cell));
                        }
                    }
                }
            }

            // Cursor
            if !screen.hide_cursor() {
                let (cur_row, cur_col) = screen.cursor_position();
                if cur_row < inner_h && cur_col < inner_w {
                    frame.set_cursor_position((rect.x + 1 + cur_col, rect.y + 1 + cur_row));
                }
            }
        })?;

        // Forward keyboard input to the PTY.
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    let bytes = key_to_bytes(key);
                    if !bytes.is_empty() {
                        let _ = pty_writer.write_all(&bytes);
                        let _ = pty_writer.flush();
                    }
                }
                Event::Resize(_, _) => {
                    if let Ok((new_rect, new_iw, new_ih)) = make_rect(width, height) {
                        rect = new_rect;
                        inner_w = new_iw;
                        inner_h = new_ih;
                        parser.set_size(inner_h, inner_w);
                        terminal.resize(rect)?;
                    }
                }
                _ => {}
            }
        }
    }

    let _ = child.wait();
    execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn cell_style(cell: &vt100::Cell) -> Style {
    let fg = vt100_color(cell.fgcolor());
    let bg = vt100_color(cell.bgcolor());
    let mut style = Style::default().fg(fg).bg(bg);
    if cell.bold() {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic() {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline() {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

pub fn vt100_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Convert a crossterm key event into the byte sequence a terminal would send.
pub fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let mut bytes: Vec<u8> = match key.code {
        KeyCode::Char(c) if ctrl => {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() {
                vec![c as u8 - b'a' + 1]
            } else if c == ' ' {
                vec![0]
            } else {
                vec![]
            }
        }
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::F(1) => vec![0x1b, b'O', b'P'],
        KeyCode::F(2) => vec![0x1b, b'O', b'Q'],
        KeyCode::F(3) => vec![0x1b, b'O', b'R'],
        KeyCode::F(4) => vec![0x1b, b'O', b'S'],
        KeyCode::F(5) => vec![0x1b, b'[', b'1', b'5', b'~'],
        KeyCode::F(6) => vec![0x1b, b'[', b'1', b'7', b'~'],
        KeyCode::F(7) => vec![0x1b, b'[', b'1', b'8', b'~'],
        KeyCode::F(8) => vec![0x1b, b'[', b'1', b'9', b'~'],
        KeyCode::F(9) => vec![0x1b, b'[', b'2', b'0', b'~'],
        KeyCode::F(10) => vec![0x1b, b'[', b'2', b'1', b'~'],
        KeyCode::F(11) => vec![0x1b, b'[', b'2', b'3', b'~'],
        KeyCode::F(12) => vec![0x1b, b'[', b'2', b'4', b'~'],
        _ => vec![],
    };

    if alt && !bytes.is_empty() && key.code != KeyCode::Esc {
        bytes.insert(0, 0x1b);
    }

    bytes
}

/// Paint captured tmux pane content as the background of the alternate screen.
fn paint_background(bg: &[u8]) -> io::Result<()> {
    let content = String::from_utf8_lossy(bg);
    let lines: Vec<&str> = content.split('\n').collect();
    let count = if lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.len() - 1
    } else {
        lines.len()
    };
    for (row, line) in lines.iter().enumerate().take(count) {
        execute!(io::stdout(), cursor::MoveTo(0, row as u16))?;
        write!(io::stdout(), "{}", line)?;
    }
    io::stdout().flush()
}

/// Capture the current tmux pane's visible content with ANSI escape sequences.
fn capture_pane_snapshot() -> Option<Vec<u8>> {
    if std::env::var("TMUX").is_err() {
        return None;
    }
    let out = std::process::Command::new("tmux")
        .args(["capture-pane", "-p", "-e"])
        .output()
        .ok()?;
    if out.status.success() && !out.stdout.is_empty() {
        Some(out.stdout)
    } else {
        None
    }
}
