use indicatif::{ProgressBar, ProgressStyle};
use ordo_core::build_graph::{BuildGraph, BuildTask};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::SystemTime;
use std::{fs, thread};

pub struct FaberEngine {
    jobs: usize,
    max_errors: usize,
}

pub struct FaberResult {
    pub success: bool,
    pub compiled: usize,
    pub skipped: usize,
    pub errors: Vec<FaberError>,
}

pub struct FaberError {
    pub source: PathBuf,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

struct TaskResult {
    #[allow(dead_code)]
    index: usize,
    source: PathBuf,
    ok: bool,
    stderr: String,
    exit_code: Option<i32>,
}

impl FaberEngine {
    pub fn new(jobs: Option<u32>, max_errors: Option<usize>) -> Self {
        let jobs = jobs
            .map(|j| j as usize)
            .unwrap_or_else(|| thread::available_parallelism().map_or(4, |n| n.get()));
        Self {
            jobs,
            max_errors: max_errors.unwrap_or(1),
        }
    }

    pub fn execute(&self, graph: &BuildGraph, verbose: u8) -> miette::Result<FaberResult> {
        eprintln!("  Using Faber build engine (beta)");

        fs::create_dir_all(&graph.build_dir)
            .map_err(|e| miette::miette!("failed to create build dir: {e}"))?;

        // Determine which tasks need rebuilding
        let mut to_build: Vec<usize> = Vec::new();
        let mut skipped: usize = 0;
        for (i, task) in graph.tasks.iter().enumerate() {
            if needs_rebuild(task, &graph.build_dir) {
                to_build.push(i);
            } else {
                skipped += 1;
            }
        }

        if to_build.is_empty() {
            // Check if link output exists
            let link_output = if graph.link.output.is_relative() {
                graph.build_dir.join(&graph.link.output)
            } else {
                graph.link.output.clone()
            };

            if link_output.exists() {
                return Ok(FaberResult {
                    success: true,
                    compiled: 0,
                    skipped,
                    errors: Vec::new(),
                });
            }
            // Need to link even though nothing compiled
        }

        let total_compile = to_build.len();
        let compiled = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));

        // Set up progress bar
        let pb = if total_compile > 0 {
            let pb = ProgressBar::new(total_compile as u64);
            pb.set_style(
                ProgressStyle::with_template("  [{bar:30.cyan/dim}] {pos}/{len} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("##-"),
            );
            pb
        } else {
            ProgressBar::hidden()
        };

        let mut errors: Vec<FaberError> = Vec::new();

        if total_compile > 0 {
            let worker_count = self.jobs.min(total_compile);
            let build_dir = graph.build_dir.clone();

            let (tx_result, rx_result) = mpsc::channel::<TaskResult>();

            let tasks_to_run: Vec<(usize, &BuildTask)> =
                to_build.iter().map(|&i| (i, &graph.tasks[i])).collect();

            let work_index = Arc::new(AtomicUsize::new(0));
            let max_errors = self.max_errors;
            let error_count_clone = Arc::clone(&error_count);

            thread::scope(|s| {
                let tasks_ref = &tasks_to_run;
                let build_dir_ref = &build_dir;

                for _ in 0..worker_count {
                    let tx = tx_result.clone();
                    let wi = Arc::clone(&work_index);
                    let ec = Arc::clone(&error_count_clone);

                    s.spawn(move || {
                        loop {
                            if ec.load(Ordering::Relaxed) >= max_errors && max_errors > 0 {
                                break;
                            }

                            let idx = wi.fetch_add(1, Ordering::Relaxed);
                            if idx >= tasks_ref.len() {
                                break;
                            }

                            let (task_index, task) = &tasks_ref[idx];
                            let result = execute_compile_task(task, build_dir_ref, verbose);
                            if !result.ok {
                                ec.fetch_add(1, Ordering::Relaxed);
                            }
                            let _ = tx.send(TaskResult {
                                index: *task_index,
                                source: task.source.clone(),
                                ok: result.ok,
                                stderr: result.stderr,
                                exit_code: result.exit_code,
                            });
                        }
                    });
                }

                // Drop sender in main thread so rx closes when all workers finish
                drop(tx_result);

                for result in &rx_result {
                    if result.ok {
                        compiled.fetch_add(1, Ordering::Relaxed);
                        let source_display = clean_source_path(&result.source);
                        pb.println(format!("  Compiled {source_display}"));
                        pb.set_message(format!("Compiling {source_display}"));
                        pb.inc(1);
                    } else {
                        pb.inc(1);
                        errors.push(FaberError {
                            source: result.source,
                            stderr: result.stderr,
                            exit_code: result.exit_code,
                        });
                    }
                }
            });
        }

        pb.finish_and_clear();

        if !errors.is_empty() {
            // Print error details
            for err in &errors {
                let source_display = clean_source_path(&err.source);
                eprintln!("  error: failed to compile {source_display}");
                if !err.stderr.is_empty() {
                    for line in err.stderr.lines() {
                        eprintln!("  {line}");
                    }
                }
            }

            return Ok(FaberResult {
                success: false,
                compiled: compiled.load(Ordering::Relaxed),
                skipped,
                errors,
            });
        }

        // Execute link task if any compile ran, or if output doesn't exist
        let any_compiled = compiled.load(Ordering::Relaxed) > 0 || total_compile > 0;
        let link_output = if graph.link.output.is_relative() {
            graph.build_dir.join(&graph.link.output)
        } else {
            graph.link.output.clone()
        };
        let need_link = any_compiled || !link_output.exists();

        if need_link && !graph.link.command.is_empty() {
            if verbose > 0 {
                eprintln!("  $ {}", graph.link.command.join(" "));
            }
            let link_result = execute_link_task(&graph.link.command, &graph.build_dir);
            if !link_result.ok {
                eprintln!("  error: linking failed");
                if !link_result.stderr.is_empty() {
                    for line in link_result.stderr.lines() {
                        eprintln!("  {line}");
                    }
                }
                return Ok(FaberResult {
                    success: false,
                    compiled: compiled.load(Ordering::Relaxed),
                    skipped,
                    errors: vec![FaberError {
                        source: PathBuf::from("<link>"),
                        stderr: link_result.stderr,
                        exit_code: link_result.exit_code,
                    }],
                });
            }
            let output_display = clean_source_path(&graph.link.output);
            eprintln!("  Linked {output_display}");
        }

        Ok(FaberResult {
            success: true,
            compiled: compiled.load(Ordering::Relaxed),
            skipped,
            errors: Vec::new(),
        })
    }
}

struct ExecResult {
    ok: bool,
    stderr: String,
    exit_code: Option<i32>,
}

fn execute_compile_task(task: &BuildTask, build_dir: &Path, verbose: u8) -> ExecResult {
    if task.command.is_empty() {
        return ExecResult {
            ok: false,
            stderr: "empty command".to_string(),
            exit_code: None,
        };
    }

    // Ensure parent directory of the object file exists
    let obj_path = if task.object.is_relative() {
        build_dir.join(&task.object)
    } else {
        task.object.clone()
    };
    if let Some(parent) = obj_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if verbose > 0 {
        eprintln!("  $ {}", task.command.join(" "));
    }

    let result = Command::new(&task.command[0])
        .args(&task.command[1..])
        .current_dir(build_dir)
        .output();

    match result {
        Ok(output) => ExecResult {
            ok: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        },
        Err(e) => ExecResult {
            ok: false,
            stderr: format!("failed to execute '{}': {e}", task.command[0]),
            exit_code: None,
        },
    }
}

fn execute_link_task(command: &[String], build_dir: &Path) -> ExecResult {
    if command.is_empty() {
        return ExecResult {
            ok: false,
            stderr: "empty link command".to_string(),
            exit_code: None,
        };
    }

    let result = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(build_dir)
        .output();

    match result {
        Ok(output) => ExecResult {
            ok: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        },
        Err(e) => ExecResult {
            ok: false,
            stderr: format!("failed to execute '{}': {e}", command[0]),
            exit_code: None,
        },
    }
}

fn clean_source_path(path: &Path) -> String {
    let s = path.display().to_string();
    let mut result = s.as_str();
    while let Some(rest) = result.strip_prefix("../") {
        result = rest;
    }
    if let Some(rest) = result.strip_prefix("./") {
        result = rest;
    }
    result.to_string()
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn needs_rebuild(task: &BuildTask, build_dir: &Path) -> bool {
    let obj_path = if task.object.is_relative() {
        build_dir.join(&task.object)
    } else {
        task.object.clone()
    };

    let obj_mtime = match file_mtime(&obj_path) {
        Some(t) => t,
        None => return true, // object doesn't exist
    };

    // Check source file
    let source_mtime = match file_mtime(&task.source) {
        Some(t) => t,
        None => return true, // source missing (will fail at compile)
    };

    if source_mtime > obj_mtime {
        return true;
    }

    // Check depfile dependencies
    let depfile_path = if task.depfile.is_relative() {
        build_dir.join(&task.depfile)
    } else {
        task.depfile.clone()
    };

    if depfile_path.exists() {
        let deps = parse_depfile(&depfile_path);
        for dep in deps {
            if let Some(dep_mtime) = file_mtime(&dep) {
                if dep_mtime > obj_mtime {
                    return true;
                }
            } else {
                // Dependency file no longer exists; rebuild
                return true;
            }
        }
    }

    false
}

fn parse_depfile(path: &Path) -> Vec<PathBuf> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Format: target: dep1 dep2 \
    //           dep3 dep4
    // We need to:
    // 1. Join backslash-continuation lines
    // 2. Strip the "target:" prefix
    // 3. Split remaining by whitespace

    let mut joined = String::new();
    for line in content.lines() {
        let trimmed = line.trim_end();
        if let Some(without_backslash) = trimmed.strip_suffix('\\') {
            joined.push_str(without_backslash);
            joined.push(' ');
        } else {
            joined.push_str(trimmed);
        }
    }

    // Find the colon that separates target from deps
    // Handle the case where the target path may contain a drive letter (e.g. C:\...)
    let deps_str = if let Some(colon_pos) = find_dep_colon(&joined) {
        &joined[colon_pos + 1..]
    } else {
        // No colon found, treat entire content as deps (unlikely but safe)
        &joined
    };

    deps_str
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(s.replace("\\ ", " ")))
        .collect()
}

/// Find the colon separating target from dependencies.
/// Skips drive-letter colons on Windows (e.g. `C:\...`).
fn find_dep_colon(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            // Skip if this looks like a Windows drive letter (single alpha char before colon)
            if i == 1 && bytes[0].is_ascii_alphabetic() {
                continue;
            }
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordo_core::build_graph::BuildTask;
    use std::fs;
    use tempfile::TempDir;

    // ---- parse_depfile tests ----

    #[test]
    fn parse_depfile_single_line() {
        let tmp = TempDir::new().unwrap();
        let dep_file = tmp.path().join("test.d");
        fs::write(&dep_file, "main.o: src/main.cpp include/foo.h").unwrap();

        let deps = parse_depfile(&dep_file);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], PathBuf::from("src/main.cpp"));
        assert_eq!(deps[1], PathBuf::from("include/foo.h"));
    }

    #[test]
    fn parse_depfile_multiline_continuation() {
        let tmp = TempDir::new().unwrap();
        let dep_file = tmp.path().join("test.d");
        fs::write(
            &dep_file,
            "main.o: src/main.cpp \\\n  include/foo.h \\\n  include/bar.h\n",
        )
        .unwrap();

        let deps = parse_depfile(&dep_file);
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], PathBuf::from("src/main.cpp"));
        assert_eq!(deps[1], PathBuf::from("include/foo.h"));
        assert_eq!(deps[2], PathBuf::from("include/bar.h"));
    }

    #[test]
    fn parse_depfile_empty() {
        let tmp = TempDir::new().unwrap();
        let dep_file = tmp.path().join("empty.d");
        fs::write(&dep_file, "").unwrap();

        let deps = parse_depfile(&dep_file);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_depfile_nonexistent() {
        let deps = parse_depfile(Path::new("/nonexistent/file.d"));
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_depfile_target_only() {
        let tmp = TempDir::new().unwrap();
        let dep_file = tmp.path().join("target_only.d");
        fs::write(&dep_file, "main.o:").unwrap();

        let deps = parse_depfile(&dep_file);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_depfile_many_deps() {
        let tmp = TempDir::new().unwrap();
        let dep_file = tmp.path().join("many.d");
        fs::write(&dep_file, "obj.o: a.cpp b.h c.h d.h e.h f.h g.h").unwrap();

        let deps = parse_depfile(&dep_file);
        assert_eq!(deps.len(), 7);
        assert_eq!(deps[0], PathBuf::from("a.cpp"));
        assert_eq!(deps[6], PathBuf::from("g.h"));
    }

    // ---- needs_rebuild tests ----

    #[test]
    fn needs_rebuild_object_missing() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"),
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        assert!(needs_rebuild(&task, tmp.path()));
    }

    #[test]
    fn needs_rebuild_source_newer() {
        let tmp = TempDir::new().unwrap();
        let obj = tmp.path().join("main.o");
        fs::write(&obj, "").unwrap();

        // Wait a tiny bit and create source so it's newer
        std::thread::sleep(std::time::Duration::from_millis(50));

        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"),
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        assert!(needs_rebuild(&task, tmp.path()));
    }

    #[test]
    fn needs_rebuild_dep_newer() {
        let tmp = TempDir::new().unwrap();

        // Create source and object
        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        let obj = tmp.path().join("main.o");
        fs::write(&obj, "").unwrap();

        // Write depfile referencing a header
        let dep_file = tmp.path().join("main.d");

        // Now create a newer header
        std::thread::sleep(std::time::Duration::from_millis(50));
        let hdr = tmp.path().join("foo.h");
        fs::write(&hdr, "// header").unwrap();

        // Depfile references the header by absolute path
        fs::write(
            &dep_file,
            format!("main.o: {} {}", src.display(), hdr.display()),
        )
        .unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"),
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        assert!(needs_rebuild(&task, tmp.path()));
    }

    #[test]
    fn needs_rebuild_up_to_date() {
        let tmp = TempDir::new().unwrap();

        // Create source
        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        // Create a header
        let hdr = tmp.path().join("foo.h");
        fs::write(&hdr, "// header").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Create object (newer than source and header)
        let obj = tmp.path().join("main.o");
        fs::write(&obj, "").unwrap();

        // Write depfile
        let dep_file = tmp.path().join("main.d");
        fs::write(
            &dep_file,
            format!("main.o: {} {}", src.display(), hdr.display()),
        )
        .unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"),
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        assert!(!needs_rebuild(&task, tmp.path()));
    }

    #[test]
    fn needs_rebuild_no_depfile() {
        let tmp = TempDir::new().unwrap();

        // Create source
        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Create object (newer than source)
        let obj = tmp.path().join("main.o");
        fs::write(&obj, "").unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"), // doesn't exist
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        // Up to date since no depfile and object is newer than source
        assert!(!needs_rebuild(&task, tmp.path()));
    }

    #[test]
    fn needs_rebuild_dep_file_deleted() {
        let tmp = TempDir::new().unwrap();

        // Create source
        let src = tmp.path().join("main.cpp");
        fs::write(&src, "int main() {}").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Create object
        let obj = tmp.path().join("main.o");
        fs::write(&obj, "").unwrap();

        // Write depfile referencing a nonexistent header
        let dep_file = tmp.path().join("main.d");
        fs::write(
            &dep_file,
            format!("main.o: {} /nonexistent/deleted.h", src.display()),
        )
        .unwrap();

        let task = BuildTask {
            source: src,
            object: PathBuf::from("main.o"),
            depfile: PathBuf::from("main.d"),
            command: vec!["cc".into(), "-c".into(), "main.cpp".into()],
            is_cpp: true,
        };

        // Should rebuild because a dependency doesn't exist
        assert!(needs_rebuild(&task, tmp.path()));
    }

    // ---- find_dep_colon tests ----

    #[test]
    fn find_dep_colon_normal() {
        assert_eq!(find_dep_colon("main.o: src/main.cpp"), Some(6));
    }

    #[test]
    fn find_dep_colon_no_colon() {
        assert_eq!(find_dep_colon("no colon here"), None);
    }
}
