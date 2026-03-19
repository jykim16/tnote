const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";

/// Render note markdown content to an ANSI-coloured string.
///
/// Supported syntax:
///   # / ## heading    → bold
///   ---               → dim horizontal rule
///   - [x] item        → green ✔  (done)
///   - [ ] item        → cyan →   (first unchecked = current), then dim (pending)
pub fn render_note(content: &str) -> String {
    let mut out = String::new();
    let mut found_current = false;

    for line in content.lines() {
        if line.starts_with("- [x]") || line.starts_with("- [X]") {
            let text = line[5..].trim();
            out.push_str(&format!("{GREEN}✔{RESET} {text}\n"));
        } else if line.starts_with("- [ ]") {
            let text = line[5..].trim();
            if !found_current {
                found_current = true;
                out.push_str(&format!("{CYAN}→{RESET} {text}\n"));
            } else {
                out.push_str(&format!("{DIM}  {text}{RESET}\n"));
            }
        } else if line == "---" {
            out.push_str(&format!("{DIM}────────────────────{RESET}\n"));
        } else if line.starts_with("## ") || line.starts_with("# ") {
            out.push_str(&format!("{BOLD}{line}{RESET}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}
