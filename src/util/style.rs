use console::Style;
use std::sync::LazyLock;

static GREEN_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().green().bold());
static RED_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().red().bold());
static YELLOW_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().yellow().bold());

pub fn status(verb: &str, message: &str) {
    eprintln!("{:>12} {message}", GREEN_BOLD.apply_to(verb));
}

pub fn status_warn(verb: &str, message: &str) {
    eprintln!("{:>12} {message}", YELLOW_BOLD.apply_to(verb));
}

pub fn status_error(verb: &str, message: &str) {
    eprintln!("{:>12} {message}", RED_BOLD.apply_to(verb));
}
