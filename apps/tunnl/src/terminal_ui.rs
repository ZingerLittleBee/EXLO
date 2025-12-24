//! Terminal UI helpers for SSH client output.
//!
//! Uses the `console` crate for proper text styling and width calculation.

use console::{measure_text_width, pad_str, style, Alignment};

use crate::config::get_tunnel_url;

/// Box width (inner content width, excluding borders)
const BOX_WIDTH: usize = 58;

/// Spinner animation frames
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Get a spinner frame by index (wraps around)
pub fn spinner_frame(index: usize) -> &'static str {
    SPINNER_FRAMES[index % SPINNER_FRAMES.len()]
}

/// Create a horizontal border line
fn top_border() -> String {
    format!("╔{}╗\r\n", "═".repeat(BOX_WIDTH + 2))
}

fn middle_border() -> String {
    format!("╠{}╣\r\n", "═".repeat(BOX_WIDTH + 2))
}

fn bottom_border() -> String {
    format!("╚{}╝\r\n", "═".repeat(BOX_WIDTH + 2))
}

/// Create a content line with proper padding using console's pad_str
fn content_line(text: &str) -> String {
    // Use console's pad_str which handles unicode width correctly
    let padded = pad_str(text, BOX_WIDTH, Alignment::Left, None);
    format!("║ {} ║\r\n", padded)
}

/// Create a centered content line
fn centered_line(text: &str) -> String {
    let padded = pad_str(text, BOX_WIDTH, Alignment::Center, None);
    format!("║ {} ║\r\n", padded)
}

/// Create an empty line
fn empty_line() -> String {
    content_line("")
}

/// Create the device activation box shown when waiting for user verification
pub fn create_activation_box(code: &str, url: &str) -> String {
    let title = format!("{} DEVICE ACTIVATION", style("🔐").yellow());

    let code_styled = format!("{}", style(code).yellow().bold());
    let code_line = format!("Your code: {}", code_styled);

    // Truncate URL if too long
    let url_display = if measure_text_width(url) > BOX_WIDTH - 2 {
        let truncated: String = url.chars().take(BOX_WIDTH - 5).collect();
        format!("{}...", truncated)
    } else {
        url.to_string()
    };
    let url_styled = format!("{}", style(&url_display).cyan().underlined());

    let spinner_line = format!("{} Waiting for authorization...", spinner_frame(0));

    let mut output = String::new();
    output.push_str("\r\n");
    output.push_str(&top_border());
    output.push_str(&centered_line(&title));
    output.push_str(&middle_border());
    output.push_str(&empty_line());
    output.push_str(&content_line(&code_line));
    output.push_str(&empty_line());
    output.push_str(&content_line("Open this URL in your browser:"));
    output.push_str(&content_line(&url_styled));
    output.push_str(&empty_line());
    output.push_str(&content_line(&spinner_line));
    output.push_str(&bottom_border());
    output.push_str("\r\n");

    output
}

/// Create the ANSI escape sequence to update the spinner line in-place
pub fn create_spinner_update(frame_index: usize) -> String {
    let spinner = spinner_frame(frame_index);
    let line_content = format!("{} Waiting for authorization...", spinner);
    let padded = pad_str(&line_content, BOX_WIDTH, Alignment::Left, None);

    // Save cursor, move up 3 lines, write the line, restore cursor
    format!("\x1B[s\x1B[3A\r║ {} ║\x1B[u", padded)
}

/// Number of lines in the activation box (for clearing)
pub const ACTIVATION_BOX_LINES: usize = 14;

/// Create the success box shown after tunnel activation
pub fn create_success_box(username: &str, tunnel_urls: &[(String, u32)]) -> String {
    let title = format!("{} TUNNEL ACTIVATED", style("✓").green());

    // Truncate username if too long
    let display_user = if username.len() > 30 {
        format!("{}...", &username[..27])
    } else {
        username.to_string()
    };
    let welcome_styled = format!("Welcome back, {}!", style(&display_user).bold());

    let disconnect_hint = format!("{}", style("Press Esc double to disconnect").dim());

    let mut output = String::new();

    // Move up and clear the old box
    output.push_str(&format!("\x1B[{}A\x1B[0J", ACTIVATION_BOX_LINES));

    output.push_str(&top_border());
    output.push_str(&centered_line(&title));
    output.push_str(&middle_border());
    output.push_str(&empty_line());
    output.push_str(&content_line(&welcome_styled));
    output.push_str(&empty_line());
    output.push_str(&content_line("Your tunnel is ready:"));

    for (subdomain, _port) in tunnel_urls {
        let full_url = get_tunnel_url(subdomain);
        let url_line = format!(
            "{} {}",
            style("➜").cyan(),
            style(&full_url).cyan().underlined()
        );
        output.push_str(&content_line(&url_line));
    }

    output.push_str(&empty_line());
    output.push_str(&content_line(&disconnect_hint));
    output.push_str(&bottom_border());
    output.push_str("\r\n");

    output
}

/// Create the error box shown when activation fails
pub fn create_error_box(reason: &str) -> String {
    let title = format!("{} ACTIVATION FAILED", style("✗").red());

    // Truncate reason if too long
    let display_reason = if reason.len() > BOX_WIDTH - 4 {
        format!("{}...", &reason[..BOX_WIDTH - 7])
    } else {
        reason.to_string()
    };
    let error_line = format!("{} {}", style("✗").red(), display_reason);

    let mut output = String::new();

    // Move up and clear the old box
    output.push_str(&format!("\x1B[{}A\x1B[0J", ACTIVATION_BOX_LINES));

    output.push_str(&top_border());
    output.push_str(&centered_line(&title));
    output.push_str(&middle_border());
    output.push_str(&empty_line());
    output.push_str(&content_line(&error_line));
    output.push_str(&empty_line());
    output.push_str(&content_line("Please reconnect to try again."));
    output.push_str(&empty_line());
    output.push_str(&content_line("Connection will close in 3 seconds..."));
    output.push_str(&bottom_border());
    output.push_str("\r\n");

    output
}

/// Create an error box for port connection failure
pub fn create_port_error_box(port: u32, address: &str) -> String {
    let title = format!("{} CONNECTION FAILED", style("✗").red());

    let error_line = format!(
        "{} Cannot connect to {}:{}",
        style("✗").red(),
        address,
        port
    );

    let mut output = String::new();

    // Move up and clear the old box
    output.push_str(&format!("\x1B[{}A\x1B[0J", ACTIVATION_BOX_LINES));

    output.push_str(&top_border());
    output.push_str(&centered_line(&title));
    output.push_str(&middle_border());
    output.push_str(&empty_line());
    output.push_str(&content_line(&error_line));
    output.push_str(&empty_line());
    output.push_str(&content_line("Make sure your local service is running:"));
    let hint = format!("  {} your-app --port {}", style("$").dim(), port);
    output.push_str(&content_line(&hint));
    output.push_str(&empty_line());
    output.push_str(&content_line("Connection will close in 3 seconds..."));
    output.push_str(&bottom_border());
    output.push_str("\r\n");

    output
}

/// Create a hint message for ESC key press
pub fn create_esc_hint() -> String {
    format!(
        "\r\n{} Press ESC again to disconnect...\r\n",
        style("⚠").yellow()
    )
}

/// Clear the ESC hint (move up and clear line)
pub fn clear_esc_hint() -> String {
    "\x1B[2A\x1B[0J".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_frame() {
        assert_eq!(spinner_frame(0), "⠋");
        assert_eq!(spinner_frame(10), "⠋"); // wraps around
    }

    #[test]
    fn test_activation_box_contains_code() {
        let box_output = create_activation_box("ABC123", "http://example.com/activate");
        assert!(box_output.contains("ABC123"));
        assert!(box_output.contains("example.com"));
    }

    #[test]
    fn test_box_width_consistency() {
        // All border lines should have the same length
        let top = top_border();
        let mid = middle_border();
        let bot = bottom_border();

        // Remove \r\n for comparison
        let top_len = measure_text_width(top.trim());
        let mid_len = measure_text_width(mid.trim());
        let bot_len = measure_text_width(bot.trim());

        assert_eq!(top_len, mid_len);
        assert_eq!(mid_len, bot_len);
    }
}
