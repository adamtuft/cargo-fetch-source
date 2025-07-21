use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process as proc;

pub fn make_progress_spinner(m: &indicatif::MultiProgress) -> indicatif::ProgressBar {
    let pb = m.add(indicatif::ProgressBar::new_spinner());
    pb.set_style(indicatif::ProgressStyle::default_spinner());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        indicatif::ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ]),
    );
    pb
}

fn make_progress_spinners(n: usize) -> (indicatif::MultiProgress, Vec<indicatif::ProgressBar>) {
    let m = indicatif::MultiProgress::new();
    let bars = (0..n).map(|_| make_progress_spinner(&m)).collect();
    (m, bars)
}

fn update_task_progress_bars(
    tasks: &mut [std::process::Child],
    bars: &[indicatif::ProgressBar],
    readers: &mut [(
        BufReader<std::process::ChildStdout>,
        BufReader<std::process::ChildStderr>,
    )],
) {
    let len = tasks.len();
    let items = std::iter::zip(tasks.iter_mut(), bars.iter()).zip(readers.iter_mut());
    for (i, ((child, pb), (stdout, stderr))) in items.enumerate() {
        match child.try_wait() {
            Ok(Some(s)) => {
                pb.finish_with_message(format!("[{}/{}] finished with status: {s}", i + 1, len));
            }
            Ok(None) => {
                let mut status = String::new();
                let _ = stderr.read_line(&mut status);
                pb.set_message(format!("[{}/{}] {}", i + 1, len, status.trim()));
            }
            Err(e) => {
                pb.finish_with_message(format!("[{}/{}] an error occurred: {e}", i + 1, len));
            }
        }
    }
}

fn finalise_remaining_bars(tasks: &mut [std::process::Child], bars: &[indicatif::ProgressBar]) {
    let len = tasks.len();
    for (i, (child, pb)) in &mut std::iter::zip(tasks.iter_mut(), bars.iter()).enumerate() {
        if !pb.is_finished() {
            match child.try_wait() {
                Ok(Some(s)) => {
                    pb.finish_with_message(format!(
                        "[{}/{}] finished with status: {s}",
                        i + 1,
                        len
                    ));
                }
                Ok(None) => {
                    pb.finish_with_message(format!("[{}/{}] should have finished!", i + 1, len));
                }
                Err(e) => {
                    pb.finish_with_message(format!("[{}/{}] an error occurred: {e}", i + 1, len));
                }
            }
        }
    }
}
