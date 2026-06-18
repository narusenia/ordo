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

pub struct BuildStep {
    pub action: String,
    pub file: String,
    pub current: u32,
    pub total: u32,
    pub done_verb: &'static str,
}

pub struct SpinnerWithDetail {
    pub mp: MultiProgress,
    pub spinner: ProgressBar,
    pub detail: ProgressBar,
}

impl SpinnerWithDetail {
    pub fn set_detail(&self, msg: &str) {
        self.detail
            .set_message(format!("  {} {}", DIM.apply_to("└"), DIM.apply_to(msg)));
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

pub trait StyleOutput {
    // --- Text output ---
    fn success(&self, verb: &str, msg: &str);
    fn error(&self, verb: &str, msg: &str);
    fn warn(&self, verb: &str, msg: &str);
    fn skip(&self, verb: &str, msg: &str);
    fn run_icon(&self, verb: &str, msg: &str);
    fn header(&self, msg: &str);
    fn meta(&self, msg: &str);
    fn tree_line(&self, line: &str);
    fn tree_detail(&self, prefix: &str, detail: &str);
    fn verbose_cmd(&self, cmd: &str);
    fn summary_bar(&self);

    // --- Spinners / progress ---
    fn create_spinner(&self, msg: &str) -> ProgressBar;
    fn create_spinner_with_detail(&self, msg: &str) -> SpinnerWithDetail;
    fn create_multi_progress(&self) -> MultiProgress;
    fn spinner_in_multi(&self, mp: &MultiProgress, prefix: &str, msg: &str) -> ProgressBar;
    fn finish_spinner_success(&self, pb: &ProgressBar, verb: &str, msg: &str);
    fn finish_spinner_error(&self, pb: &ProgressBar, verb: &str, msg: &str);

    // --- Build step display ---
    fn display_build_step(&self, step: &BuildStep, spinner: &ProgressBar);
    fn finish_build_step(&self, step: &BuildStep);
    fn finish_build_failed(&self, step: &BuildStep);
}

// === DefaultStyle ===

pub struct DefaultStyle;

impl StyleOutput for DefaultStyle {
    fn success(&self, verb: &str, msg: &str) {
        if verb.is_empty() {
            eprintln!("{} {msg}", GREEN_BOLD.apply_to(ICON_SUCCESS));
        } else {
            eprintln!(
                "{} {} {msg}",
                GREEN_BOLD.apply_to(ICON_SUCCESS),
                GREEN_BOLD.apply_to(verb)
            );
        }
    }

    fn error(&self, verb: &str, msg: &str) {
        if verb.is_empty() {
            eprintln!("{} {msg}", RED_BOLD.apply_to(ICON_FAILURE));
        } else {
            eprintln!(
                "{} {} {msg}",
                RED_BOLD.apply_to(ICON_FAILURE),
                RED_BOLD.apply_to(verb)
            );
        }
    }

    fn warn(&self, verb: &str, msg: &str) {
        eprintln!(
            "{} {} {msg}",
            YELLOW_BOLD.apply_to(ICON_WARNING),
            YELLOW_BOLD.apply_to(verb)
        );
    }

    fn skip(&self, verb: &str, msg: &str) {
        eprintln!("{} {} {msg}", DIM.apply_to(ICON_SKIP), DIM.apply_to(verb));
    }

    fn run_icon(&self, verb: &str, msg: &str) {
        eprintln!(
            "{} {} {msg}",
            GREEN_BOLD.apply_to(ICON_RUN),
            GREEN_BOLD.apply_to(verb)
        );
    }

    fn header(&self, msg: &str) {
        eprintln!();
        eprintln!("{} {}", CYAN_BOLD.apply_to("▸"), CYAN_BOLD.apply_to(msg));
    }

    fn meta(&self, msg: &str) {
        eprintln!("  {}", DIM.apply_to(msg));
    }

    fn tree_line(&self, line: &str) {
        eprintln!("  {}", DIM.apply_to(line));
    }

    fn tree_detail(&self, prefix: &str, detail: &str) {
        eprintln!("{prefix}{}", DIM.apply_to(detail));
    }

    fn verbose_cmd(&self, cmd: &str) {
        eprintln!("  {}", DIM.apply_to(format!("$ {cmd}")));
    }

    fn summary_bar(&self) {
        eprintln!(
            "{}",
            DIM.apply_to("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        );
    }

    fn create_spinner(&self, msg: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
                .template(&format!(
                    "{{spinner}} {} {{msg}}",
                    CYAN_BOLD.apply_to("{prefix}")
                ))
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message(msg.to_string());
        pb
    }

    fn create_spinner_with_detail(&self, msg: &str) -> SpinnerWithDetail {
        let mp = MultiProgress::new();

        let spinner = mp.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
                .template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner.set_message(msg.to_string());

        let detail = mp.add(ProgressBar::new_spinner());
        detail.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        detail.enable_steady_tick(Duration::from_millis(80));

        SpinnerWithDetail {
            mp,
            spinner,
            detail,
        }
    }

    fn create_multi_progress(&self) -> MultiProgress {
        MultiProgress::new()
    }

    fn spinner_in_multi(&self, mp: &MultiProgress, prefix: &str, msg: &str) -> ProgressBar {
        let style = ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""])
            .template("{spinner} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());

        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(style);
        pb.set_message(format!("{} {msg}", CYAN_BOLD.apply_to(prefix)));
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    fn finish_spinner_success(&self, pb: &ProgressBar, verb: &str, msg: &str) {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{wide_msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.finish_with_message(format!(
            "{} {} {msg}",
            GREEN_BOLD.apply_to(ICON_SUCCESS),
            GREEN_BOLD.apply_to(verb),
        ));
    }

    fn finish_spinner_error(&self, pb: &ProgressBar, verb: &str, msg: &str) {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{wide_msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.finish_with_message(format!(
            "{} {} {msg}",
            RED_BOLD.apply_to(ICON_FAILURE),
            RED_BOLD.apply_to(verb),
        ));
    }

    fn display_build_step(&self, step: &BuildStep, spinner: &ProgressBar) {
        let progress = format!("[{}/{}]", step.current, step.total);
        spinner.reset();
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner.set_message(format!("{} {} {progress}", step.action, step.file));
    }

    fn finish_build_step(&self, step: &BuildStep) {
        let progress = format!("[{}/{}]", step.current, step.total);
        self.success(step.done_verb, &format!("{} {progress}", step.file));
    }

    fn finish_build_failed(&self, step: &BuildStep) {
        let progress = format!("[{}/{}]", step.current, step.total);
        self.error("Failed", &format!("{} {progress}", step.file));
    }
}

// === MinimalStyle ===

pub struct MinimalStyle;

impl StyleOutput for MinimalStyle {
    fn success(&self, verb: &str, msg: &str) {
        if verb.is_empty() {
            eprintln!("{msg}");
        } else {
            eprintln!("{verb} {msg}");
        }
    }

    fn error(&self, verb: &str, msg: &str) {
        if verb.is_empty() {
            eprintln!("{msg}");
        } else {
            eprintln!("{verb} {msg}");
        }
    }

    fn warn(&self, verb: &str, msg: &str) {
        eprintln!("{verb} {msg}");
    }

    fn skip(&self, verb: &str, msg: &str) {
        eprintln!("{verb} {msg}");
    }

    fn run_icon(&self, verb: &str, msg: &str) {
        eprintln!("{verb} {msg}");
    }

    fn header(&self, msg: &str) {
        eprintln!("{msg}");
    }

    fn meta(&self, msg: &str) {
        eprintln!("  {msg}");
    }

    fn tree_line(&self, line: &str) {
        eprintln!("  {line}");
    }

    fn tree_detail(&self, prefix: &str, detail: &str) {
        eprintln!("{prefix}{detail}");
    }

    fn verbose_cmd(&self, cmd: &str) {
        eprintln!("  $ {cmd}");
    }

    fn summary_bar(&self) {}

    fn create_spinner(&self, _msg: &str) -> ProgressBar {
        ProgressBar::hidden()
    }

    fn create_spinner_with_detail(&self, msg: &str) -> SpinnerWithDetail {
        eprintln!("{msg}");
        SpinnerWithDetail {
            mp: MultiProgress::new(),
            spinner: ProgressBar::hidden(),
            detail: ProgressBar::hidden(),
        }
    }

    fn create_multi_progress(&self) -> MultiProgress {
        MultiProgress::new()
    }

    fn spinner_in_multi(&self, mp: &MultiProgress, _prefix: &str, _msg: &str) -> ProgressBar {
        mp.add(ProgressBar::hidden())
    }

    fn finish_spinner_success(&self, _pb: &ProgressBar, verb: &str, msg: &str) {
        self.success(verb, msg);
    }

    fn finish_spinner_error(&self, _pb: &ProgressBar, verb: &str, msg: &str) {
        self.error(verb, msg);
    }

    fn display_build_step(&self, step: &BuildStep, _spinner: &ProgressBar) {
        eprintln!(
            "[{}/{}] {} {}",
            step.current, step.total, step.action, step.file
        );
    }

    fn finish_build_step(&self, _step: &BuildStep) {}

    fn finish_build_failed(&self, _step: &BuildStep) {}
}

// === CargoLikeStyle ===

pub struct CargoLikeStyle;

impl CargoLikeStyle {
    fn cargo_line(verb: &str, msg: &str) {
        eprintln!("{} {msg}", GREEN_BOLD.apply_to(format!("{verb:>12}")));
    }
}

impl StyleOutput for CargoLikeStyle {
    fn success(&self, verb: &str, msg: &str) {
        CargoLikeStyle::cargo_line(verb, msg);
    }

    fn error(&self, verb: &str, msg: &str) {
        if verb.is_empty() {
            eprintln!("{}", RED_BOLD.apply_to("error"));
        } else {
            eprintln!("{} {msg}", RED_BOLD.apply_to(format!("{verb:>12}")));
        }
    }

    fn warn(&self, verb: &str, msg: &str) {
        eprintln!("{} {msg}", YELLOW_BOLD.apply_to(format!("{verb:>12}")));
    }

    fn skip(&self, verb: &str, msg: &str) {
        CargoLikeStyle::cargo_line(verb, msg);
    }

    fn run_icon(&self, verb: &str, msg: &str) {
        CargoLikeStyle::cargo_line(verb, msg);
    }

    fn header(&self, _msg: &str) {}

    fn meta(&self, msg: &str) {
        eprintln!("{msg:>12}", msg = "");
        eprintln!("             {msg}");
    }

    fn tree_line(&self, line: &str) {
        eprintln!("  {line}");
    }

    fn tree_detail(&self, prefix: &str, detail: &str) {
        eprintln!("{prefix}{detail}");
    }

    fn verbose_cmd(&self, cmd: &str) {
        eprintln!("  $ {cmd}");
    }

    fn summary_bar(&self) {}

    fn create_spinner(&self, _msg: &str) -> ProgressBar {
        ProgressBar::hidden()
    }

    fn create_spinner_with_detail(&self, _msg: &str) -> SpinnerWithDetail {
        SpinnerWithDetail {
            mp: MultiProgress::new(),
            spinner: ProgressBar::hidden(),
            detail: ProgressBar::hidden(),
        }
    }

    fn create_multi_progress(&self) -> MultiProgress {
        MultiProgress::new()
    }

    fn spinner_in_multi(&self, mp: &MultiProgress, _prefix: &str, _msg: &str) -> ProgressBar {
        mp.add(ProgressBar::hidden())
    }

    fn finish_spinner_success(&self, _pb: &ProgressBar, verb: &str, msg: &str) {
        CargoLikeStyle::cargo_line(verb, msg);
    }

    fn finish_spinner_error(&self, _pb: &ProgressBar, verb: &str, msg: &str) {
        self.error(verb, msg);
    }

    fn display_build_step(&self, step: &BuildStep, _spinner: &ProgressBar) {
        CargoLikeStyle::cargo_line(&step.action, &step.file);
    }

    fn finish_build_step(&self, _step: &BuildStep) {}

    fn finish_build_failed(&self, _step: &BuildStep) {}
}
