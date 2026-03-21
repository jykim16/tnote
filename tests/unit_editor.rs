use tnote::editor::{key_to_bytes, vt100_color};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

// ── key_to_bytes ─────────────────────────────────────────────────────────────

#[test]
fn test_char_keys() {
    assert_eq!(key_to_bytes(key(KeyCode::Char('a'))), b"a");
    assert_eq!(key_to_bytes(key(KeyCode::Char('Z'))), b"Z");
    assert_eq!(key_to_bytes(key(KeyCode::Char('5'))), b"5");
}

#[test]
fn test_unicode_char() {
    let bytes = key_to_bytes(key(KeyCode::Char('é')));
    assert_eq!(bytes, "é".as_bytes());
}

#[test]
fn test_ctrl_chars() {
    assert_eq!(key_to_bytes(ctrl('a')), vec![1]);
    assert_eq!(key_to_bytes(ctrl('c')), vec![3]);
    assert_eq!(key_to_bytes(ctrl('z')), vec![26]);
}

#[test]
fn test_ctrl_space() {
    assert_eq!(key_to_bytes(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL)), vec![0]);
}

#[test]
fn test_ctrl_non_alpha_is_empty() {
    assert_eq!(key_to_bytes(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL)), vec![]);
}

#[test]
fn test_enter() {
    assert_eq!(key_to_bytes(key(KeyCode::Enter)), vec![b'\r']);
}

#[test]
fn test_backspace() {
    assert_eq!(key_to_bytes(key(KeyCode::Backspace)), vec![0x7f]);
}

#[test]
fn test_delete() {
    assert_eq!(key_to_bytes(key(KeyCode::Delete)), vec![0x1b, b'[', b'3', b'~']);
}

#[test]
fn test_escape() {
    assert_eq!(key_to_bytes(key(KeyCode::Esc)), vec![0x1b]);
}

#[test]
fn test_tab() {
    assert_eq!(key_to_bytes(key(KeyCode::Tab)), vec![b'\t']);
}

#[test]
fn test_back_tab() {
    assert_eq!(key_to_bytes(key(KeyCode::BackTab)), vec![0x1b, b'[', b'Z']);
}

#[test]
fn test_arrow_keys() {
    assert_eq!(key_to_bytes(key(KeyCode::Up)),    vec![0x1b, b'[', b'A']);
    assert_eq!(key_to_bytes(key(KeyCode::Down)),  vec![0x1b, b'[', b'B']);
    assert_eq!(key_to_bytes(key(KeyCode::Right)), vec![0x1b, b'[', b'C']);
    assert_eq!(key_to_bytes(key(KeyCode::Left)),  vec![0x1b, b'[', b'D']);
}

#[test]
fn test_home_end() {
    assert_eq!(key_to_bytes(key(KeyCode::Home)), vec![0x1b, b'[', b'H']);
    assert_eq!(key_to_bytes(key(KeyCode::End)),  vec![0x1b, b'[', b'F']);
}

#[test]
fn test_page_keys() {
    assert_eq!(key_to_bytes(key(KeyCode::PageUp)),   vec![0x1b, b'[', b'5', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::PageDown)), vec![0x1b, b'[', b'6', b'~']);
}

#[test]
fn test_function_keys() {
    assert_eq!(key_to_bytes(key(KeyCode::F(1))),  vec![0x1b, b'O', b'P']);
    assert_eq!(key_to_bytes(key(KeyCode::F(2))),  vec![0x1b, b'O', b'Q']);
    assert_eq!(key_to_bytes(key(KeyCode::F(3))),  vec![0x1b, b'O', b'R']);
    assert_eq!(key_to_bytes(key(KeyCode::F(4))),  vec![0x1b, b'O', b'S']);
    assert_eq!(key_to_bytes(key(KeyCode::F(5))),  vec![0x1b, b'[', b'1', b'5', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(6))),  vec![0x1b, b'[', b'1', b'7', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(7))),  vec![0x1b, b'[', b'1', b'8', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(8))),  vec![0x1b, b'[', b'1', b'9', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(9))),  vec![0x1b, b'[', b'2', b'0', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(10))), vec![0x1b, b'[', b'2', b'1', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(11))), vec![0x1b, b'[', b'2', b'3', b'~']);
    assert_eq!(key_to_bytes(key(KeyCode::F(12))), vec![0x1b, b'[', b'2', b'4', b'~']);
}

#[test]
fn test_alt_prepends_escape() {
    let bytes = key_to_bytes(alt(KeyCode::Char('b')));
    assert_eq!(bytes[0], 0x1b);
    assert_eq!(&bytes[1..], b"b");
}

#[test]
fn test_alt_esc_no_double_escape() {
    let bytes = key_to_bytes(alt(KeyCode::Esc));
    assert_eq!(bytes, vec![0x1b]);
}

#[test]
fn test_unknown_key_empty() {
    assert_eq!(key_to_bytes(key(KeyCode::F(20))), vec![]);
}

// ── vt100_color ──────────────────────────────────────────────────────────────

#[test]
fn test_vt100_color_default() {
    assert_eq!(vt100_color(vt100::Color::Default), ratatui::style::Color::Reset);
}

#[test]
fn test_vt100_color_indexed() {
    assert_eq!(vt100_color(vt100::Color::Idx(42)), ratatui::style::Color::Indexed(42));
}

#[test]
fn test_vt100_color_rgb() {
    assert_eq!(
        vt100_color(vt100::Color::Rgb(10, 20, 30)),
        ratatui::style::Color::Rgb(10, 20, 30)
    );
}
