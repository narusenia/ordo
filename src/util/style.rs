#![allow(dead_code)]

use console::Style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::LazyLock;
use std::time::Duration;

static GREEN_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().green().bold());
static CYAN_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().cyan().bold());
static RED_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().red().bold());
static YELLOW_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().yellow().bold());
static DIM: LazyLock<Style> = LazyLock::new(|| Style::new().dim());

// Icons
pub const ICON_SUCCESS: &str = "✔";
pub const ICON_FAILURE: &str = "✖";
pub const ICON_WARNING: &str = "⚠";
pub const ICON_SKIP: &str = "⊘";
pub const ICON_RUN: &str = "▶";

pub fn success(verb: &str, message: &str) {
    if verb.is_empty() {
        eprintln!("{} {message}", GREEN_BOLD.apply_to(ICON_SUCCESS));
    } else {
        eprintln!("{} {} {message}", GREEN_BOLD.apply_to(ICON_SUCCESS), GREEN_BOLD.apply_to(verb));
    }
}

pub fn error(verb: &str, message: &str) {
    if verb.is_empty() {
        eprintln!("{} {message}", RED_BOLD.apply_to(ICON_FAILURE));
    } else {
        eprintln!("{} {} {message}", RED_BOLD.apply_to(ICON_FAILURE), RED_BOLD.apply_to(verb));
    }
}

pub fn warn(verb: &str, message: &str) {
    eprintln!("{} {} {message}", YELLOW_BOLD.apply_to(ICON_WARNING), YELLOW_BOLD.apply_to(verb));
}

pub fn skip(verb: &str, message: &str) {
    eprintln!("{} {} {message}", DIM.apply_to(ICON_SKIP), DIM.apply_to(verb));
}

pub fn run_icon(verb: &str, message: &str) {
    eprintln!("{} {} {message}", GREEN_BOLD.apply_to(ICON_RUN), GREEN_BOLD.apply_to(verb));
}

pub fn meta(message: &str) {
    eprintln!("  {}", DIM.apply_to(message));
}

pub fn tree_line(line: &str) {
    eprintln!("  {}", DIM.apply_to(line));
}

pub fn verbose_cmd(cmd: &str) {
    eprintln!("  {}", DIM.apply_to(format!("$ {cmd}")));
}

pub fn summary_bar() {
    eprintln!("{}", DIM.apply_to("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
            .template(&format!("{{spinner}} {} {{msg}}", CYAN_BOLD.apply_to("{prefix}")))
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(message.to_string());
    pb
}

pub struct SpinnerWithDetail {
    pub mp: MultiProgress,
    pub spinner: ProgressBar,
    pub detail: ProgressBar,
}

impl SpinnerWithDetail {
    pub fn set_detail(&self, msg: &str) {
        self.detail.set_message(format!("  {} {}", DIM.apply_to("└"), DIM.apply_to(msg)));
    }

    pub fn finish_success(&self, verb: &str, message: &str) {
        self.detail.finish_and_clear();
        self.spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{wide_msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        self.spinner.finish_with_message(format!(
            "{} {} {message}",
            GREEN_BOLD.apply_to(ICON_SUCCESS),
            GREEN_BOLD.apply_to(verb),
        ));
    }

    pub fn finish_error(&self, verb: &str, message: &str) {
        self.detail.finish_and_clear();
        self.spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{wide_msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        self.spinner.finish_with_message(format!(
            "{} {} {message}",
            RED_BOLD.apply_to(ICON_FAILURE),
            RED_BOLD.apply_to(verb),
        ));
    }
}

pub fn create_spinner_with_detail(message: &str) -> SpinnerWithDetail {
    let mp = MultiProgress::new();

    let spinner = mp.add(ProgressBar::new_spinner());
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
            .template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message(message.to_string());

    let detail = mp.add(ProgressBar::new_spinner());
    detail.set_style(
        ProgressStyle::default_spinner()
            .template("{msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    detail.enable_steady_tick(Duration::from_millis(80));

    SpinnerWithDetail { mp, spinner, detail }
}

pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

pub fn spinner_in_multi(mp: &MultiProgress, prefix: &str, message: &str) -> ProgressBar {
    let style = ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
        .template("{spinner} {wide_msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());

    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(style);
    pb.set_message(format!("{} {message}", CYAN_BOLD.apply_to(prefix)));
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn finish_spinner_success(pb: &ProgressBar, verb: &str, message: &str) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.finish_with_message(format!(
        "{} {} {message}",
        GREEN_BOLD.apply_to(ICON_SUCCESS),
        GREEN_BOLD.apply_to(verb),
    ));
}

pub fn finish_spinner_error(pb: &ProgressBar, verb: &str, message: &str) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.finish_with_message(format!(
        "{} {} {message}",
        RED_BOLD.apply_to(ICON_FAILURE),
        RED_BOLD.apply_to(verb),
    ));
}
