use crate::Fetch;
use crate::artefact::Artefact;
use crate::git2_ext::{Repository, RepositoryExt};

#[derive(Debug, serde::Deserialize)]
enum GitReference {
    #[serde(rename = "branch")]
    Branch(String),
    #[serde(rename = "tag")]
    Tag(String),
    #[serde(rename = "rev")]
    Rev(String),
}

impl GitReference {
    fn as_branch_name(&self) -> Option<&str> {
        match self {
            GitReference::Branch(name) | GitReference::Tag(name) => Some(name),
            GitReference::Rev(_) => None,
        }
    }
    fn as_commit_sha(&self) -> Option<&str> {
        match self {
            GitReference::Branch(name) | GitReference::Tag(name) => None,
            GitReference::Rev(commit_sha) => Some(commit_sha),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GitSource {
    #[serde(rename = "git")]
    url: String,
    #[serde(flatten)]
    reference: Option<GitReference>,
    #[serde(default)]
    recursive: bool,
}

impl GitSource {
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    fn branch_name(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => Some(name),
            _ => None,
        }
    }

    fn commit_sha(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
        }
    }
}

impl std::fmt::Display for GitSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)?;
        if let Some(reference) = &self.reference {
            match reference {
                GitReference::Branch(branch) => write!(f, " (branch: {branch})")?,
                GitReference::Tag(tag) => write!(f, " (tag: {tag})")?,
                GitReference::Rev(rev) => write!(f, " (rev: {rev})")?,
            }
        }
        if self.recursive {
            write!(f, " [recursive]")?;
        }
        Ok(())
    }
}

impl Fetch for GitSource {
    fn fetch(&self, name: &str, dir: std::path::PathBuf) -> Result<Artefact, crate::Error> {
        let repo = Repository::clone_into(&self.url, self.branch_name(), &dir.join(name))?;
        if let Some(commit_sha) = self.commit_sha() {
            repo.checkout_commit(commit_sha)?;
        }
        if self.recursive {
            repo.update_submodules_recursive()?;
        }
        Ok(Artefact::Repository(repo))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process as proc;
    use std::io::{BufReader, BufRead};

    fn git_clone(url: &str, name: &str, reference: Option<GitReference>, recursive: bool) -> proc::Command {
        let mut git = proc::Command::new("git");
        git.args(["clone", "--depth", "1", "--no-tags"]);
        match &reference {
            Some(GitReference::Branch(s)) | Some(GitReference::Tag(s)) => {
                git.args(["--branch", s]);
            }
            Some(GitReference::Rev(s)) => {
                git.args(["--revision", s]);
            }
            None => {}
        }
        if recursive {
            git.arg("--recurse-submodules")
            .arg("--shallow-submodules");
        }
        git.arg(url)
            .arg(format!("test/test_git_clone_subprocess/{name}"));
        git
    }

    fn git_clone_task<P: AsRef<Path>>(source: &GitSource, into: P, pipe: bool) -> proc::Command {
        use GitReference as Ref;
        let mut git = proc::Command::new("git");
        git.args(["clone", "--depth", "1", "--no-tags"]);
        match &source.reference {
            Some(Ref::Branch(s)) | Some(Ref::Tag(s)) => {
                git.args(["--branch", s]);
            }
            Some(Ref::Rev(s)) => {
                git.args(["--revision", s]);
            }
            None => {}
        }
        if source.recursive {
            git.args(["--recurse-submodules", "--shallow-submodules"]);
        }
        git.arg(&source.url).arg(into.as_ref());
        if pipe {
            git.stdout(proc::Stdio::piped())
            .stderr(proc::Stdio::piped())
            .stdin(proc::Stdio::null());
        }
        git
    }

    fn spawn_tasks(
        sources: &std::collections::HashMap<String, crate::source::Source>, 
        root: PathBuf,
        pipe: bool
    ) -> impl Iterator<Item = std::io::Result<std::process::Child>> + '_ {
        sources.iter().filter_map(move |(n, s)| {
            if let crate::source::Source::Git(src) = s {
                Some(git_clone_task(src, root.join(n), pipe).spawn())
            } else {
                None
            }
        })
    }

    fn count_pending(tasks: &mut [std::process::Child]) -> usize {
        tasks.iter_mut().filter_map(|t| matches!(t.try_wait(), Ok(None)).then_some(t)).count()
    }

    fn tasks_pending(tasks: &mut [std::process::Child]) -> bool {
        tasks.iter_mut().any(|t| matches!(t.try_wait(), Ok(None)))
    }

    fn make_progress_spinner(m: &indicatif::MultiProgress) -> indicatif::ProgressBar {
        let pb = m.add(indicatif::ProgressBar::new_spinner());
        pb.set_style(indicatif::ProgressStyle::default_spinner());
        pb.enable_steady_tick(std::time::Duration::from_millis(120));
        pb.set_style(indicatif::ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ])
        );
        pb
    }
    
    #[test]
    fn report_task_progress() {
        let m = indicatif::MultiProgress::new();
        let bars: Vec<_> = (0..4)
            .map(|_| make_progress_spinner(&m))
            .collect();
        std::thread::sleep(std::time::Duration::from_secs(5));
        bars.into_iter().for_each(|pb| {
            pb.finish_with_message("Done");
        });
    }

    fn get_sources() -> crate::source::Sources {
        let document = std::fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>()
            .unwrap();
        crate::source::get_remote_sources_from_toml_table(&document).unwrap()
    }

    fn make_progress_spinners(n: usize) -> (indicatif::MultiProgress, Vec<indicatif::ProgressBar>) {
        let m = indicatif::MultiProgress::new();
        let bars = (0..n)
            .map(|_| make_progress_spinner(&m))
            .collect();
        (m, bars)
    }

    fn update_task_progress_bars(tasks: &mut [std::process::Child], bars: &[indicatif::ProgressBar], readers: &mut [(BufReader<std::process::ChildStdout>, BufReader<std::process::ChildStderr>)]) {
        let len = tasks.len();
        let items = std::iter::zip(tasks.iter_mut(), bars.iter()).zip(readers.iter_mut());
        for (i, ((child, pb), (stdout, stderr))) in items.enumerate() {
            match child.try_wait() {
                Ok(Some(s)) => {
                    pb.finish_with_message(format!("[{}/{}] finished with status: {s}", i+1, len));
                }
                Ok(None) => {
                    let mut status = String::new();
                    let _ = stderr.read_line(&mut status);
                    pb.set_message(format!("[{}/{}] {}", i + 1, len, status.trim()));
                }
                Err(e) => {
                    pb.finish_with_message(format!("[{}/{}] an error occurred: {e}", i+1, len));
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
                        pb.finish_with_message(format!("[{}/{}] finished with status: {s}", i+1, len));
                    }
                    Ok(None) => {
                        pb.finish_with_message(format!("[{}/{}] should have finished!", i + 1, len));
                    }
                    Err(e) => {
                        pb.finish_with_message(format!("[{}/{}] an error occurred: {e}", i+1, len));
                    }
                }
            }
        }
    }

    #[test]
    fn test_git_clone_task_progress() {
        let sources = get_sources();
        let mut tasks: Vec<_> = spawn_tasks(&sources, PathBuf::from("test/test_git_clone_task_progress"), true)
            .collect::<Result<_, _>>().unwrap();
        println!("Start {} tasks", tasks.len());
        let (m, bars) = make_progress_spinners(tasks.len());
        let mut readers = tasks.iter_mut()
            .map(|c| (BufReader::new(c.stdout.take().expect("Failed to take stdout")), BufReader::new(c.stderr.take().expect("Failed to take stderr"))))
            .collect::<Vec<_>>();
        assert_eq!(tasks.len(), bars.len());
        while tasks_pending(&mut tasks) {
            update_task_progress_bars(&mut tasks, &bars, &mut readers);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        finalise_remaining_bars(&mut tasks, &bars);
        println!();
        println!();
        println!();
    }

    #[test]
    fn test_git_clone_subprocess() {
        let sources = get_sources();
        let mut tasks: Vec<_> = spawn_tasks(&sources, PathBuf::from("test/test_git_clone_subprocess"), false)
            .collect::<Result<_, _>>().unwrap();
        println!("Waiting for {} tasks", tasks.len());
        while count_pending(&mut tasks) > 0 {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        println!("Tasks complete:\n");
        for child in &tasks {
            println!("Child: {child:?}");
        }
        for mut child in tasks {
            if let Ok(Some(status)) = child.try_wait() {
                println!("Git clone task completed with status: {status:?}");
                assert!(status.success(), "Git clone task failed: {status:?}");
            } else {
                panic!("Git clone task did not complete successfully");
            }
        }
    }
}
