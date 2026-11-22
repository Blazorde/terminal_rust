//! Background jobs (see ADR-0013): programs started with `&`, and the `jobs`,
//! `fg`, `wait` and `kill` commands that manage them.

use std::process::{Child, ExitStatus};

use crate::commands::external;
use crate::utils::job_control::{self, Resume, Waited};

/// One background pipeline: its programs and the text that started it.
struct Job {
    id: u32,
    text: String,
    /// The programs of the pipeline.
    children: Vec<Child>,
    /// What each program ended with, once it has.
    finished: Vec<Option<ExitStatus>>,
    /// The process group of the programs, when they run under job control.
    group: Option<i32>,
    /// Stopped by `Ctrl+Z`: `fg` or `bg` continue it.
    stopped: bool,
    /// How the programs left the terminal, put back when the job is continued.
    modes: Option<job_control::Modes>,
}

/// What bringing a job to the foreground came to.
enum Resumed {
    Done(Option<ExitStatus>),
    Stopped,
}

impl Job {
    /// The pid of the last program, the one the shell reports.
    fn pid(&self) -> u32 {
        self.children.last().map_or(0, Child::id)
    }

    /// Look at the programs without waiting. True when all of them are over.
    fn poll(&mut self) -> bool {
        for (index, child) in self.children.iter_mut().enumerate() {
            if self.finished[index].is_none() {
                self.finished[index] = child.try_wait().ok().flatten();
            }
        }
        self.finished.iter().all(Option::is_some)
    }

    /// Wait for every program, and return the status of the last one.
    fn wait(&mut self) -> Option<ExitStatus> {
        for (index, child) in self.children.iter_mut().enumerate() {
            if self.finished[index].is_none() {
                self.finished[index] = child.wait().ok();
            }
        }
        self.status()
    }

    /// The status the job ended with: the last program's.
    fn status(&self) -> Option<ExitStatus> {
        self.finished.last().copied().flatten()
    }

    /// Bring the job to the foreground and wait for it: it gets the terminal and is
    /// continued. Without a process group (job control is off) it is just waited for.
    fn run_in_foreground(&mut self) -> Resumed {
        let Some(group) = self.group.filter(|_| job_control::enabled()) else {
            return Resumed::Done(self.wait());
        };
        let children = std::mem::take(&mut self.children);
        let modes = self.modes.take();
        let _ = group;
        match job_control::wait_foreground(
            children,
            Some(Resume {
                modes: modes.as_ref(),
            }),
        ) {
            Waited::Done(statuses) => {
                self.finished = statuses.into_iter().map(Some).collect();
                self.stopped = false;
                Resumed::Done(self.status())
            }
            Waited::Stopped(stopped) => {
                self.children = stopped.children;
                self.modes = stopped.modes;
                self.finished = vec![None; self.children.len()];
                self.stopped = true;
                Resumed::Stopped
            }
        }
    }

    fn kill(&mut self) {
        for (index, child) in self.children.iter_mut().enumerate() {
            if self.finished[index].is_none() {
                let _ = child.kill();
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Jobs {
    list: Vec<Job>,
    next_id: u32,
}

/// How a job ended, for the notice and for `$?`.
pub(super) fn describe(status: Option<ExitStatus>) -> (String, i32) {
    match status {
        Some(status) if !status.success() => {
            let code = external::exit_code(status);
            (format!("Exit {code}"), code)
        }
        _ => ("Done".to_string(), 0),
    }
}

impl Jobs {
    /// Register a started pipeline and return its number and the pid to report.
    pub(super) fn add(
        &mut self,
        text: &str,
        children: Vec<Child>,
        group: Option<i32>,
    ) -> (u32, u32) {
        self.next_id += 1;
        let finished = vec![None; children.len()];
        let job = Job {
            id: self.next_id,
            text: text.trim().to_string(),
            children,
            finished,
            group,
            stopped: false,
            modes: None,
        };
        let reported = (job.id, job.pid());
        self.list.push(job);
        reported
    }

    /// Take in the groups that `Ctrl+Z` stopped since the last call, as jobs, and return
    /// the notices to show (`[1]+  Stopped  vim notes`).
    pub(super) fn adopt_stopped(&mut self) -> Vec<String> {
        let mut notices = Vec::new();
        for (text, stopped) in job_control::take_stopped() {
            self.next_id += 1;
            let job = Job {
                id: self.next_id,
                text: text.trim().to_string(),
                finished: vec![None; stopped.children.len()],
                children: stopped.children,
                group: Some(stopped.group),
                stopped: true,
                modes: stopped.modes,
            };
            notices.push(line(&job, Some(job.id), "Stopped"));
            self.list.push(job);
        }
        notices
    }

    /// Whether a job is stopped, which `exit` warns about.
    pub(super) fn has_stopped(&self) -> bool {
        self.list.iter().any(|job| job.stopped)
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The notices for the jobs that ended since the last call, and forget them.
    pub(super) fn finished_notices(&mut self) -> Vec<String> {
        let mut notices = Vec::new();
        let current = self.list.last().map(|job| job.id);
        self.list.retain_mut(|job| {
            if !job.poll() {
                return true;
            }
            notices.push(line(job, current, &describe(job.status()).0));
            false
        });
        notices
    }

    /// The lines `jobs` prints. A job that ended is reported as such and
    /// forgotten, as the notice would have done.
    pub(super) fn listing(&mut self) -> Vec<String> {
        let current = self.list.last().map(|job| job.id);
        let mut lines = Vec::new();
        self.list.retain_mut(|job| {
            let over = job.poll();
            let label = if over {
                describe(job.status()).0
            } else if job.stopped {
                "Stopped".to_string()
            } else {
                "Running".to_string()
            };
            lines.push(line(job, current, &label));
            !over
        });
        lines
    }

    /// Find a job by number, or the most recent one.
    fn index_of(&self, spec: Option<&str>) -> Result<usize, String> {
        let Some(spec) = spec else {
            return self
                .list
                .len()
                .checked_sub(1)
                .ok_or_else(|| "no current job".to_string());
        };
        let number = spec
            .strip_prefix('%')
            .unwrap_or(spec)
            .parse::<u32>()
            .map_err(|_| format!("{spec}: invalid job specification"))?;
        self.list
            .iter()
            .position(|job| job.id == number)
            .ok_or_else(|| format!("%{number}: no such job"))
    }

    /// Bring one job (or the most recent) to the foreground and wait for it.
    /// `announce` gets the text of the job before the wait, as `fg` shows the command.
    pub(super) fn foreground(
        &mut self,
        spec: Option<&str>,
        announce: impl FnOnce(&str),
    ) -> Result<Foreground, String> {
        let index = self.index_of(spec)?;
        let mut job = self.list.remove(index);
        announce(&job.text);
        match job.run_in_foreground() {
            Resumed::Done(status) => Ok(Foreground::Done(status)),
            Resumed::Stopped => {
                let notice = line(&job, Some(job.id), "Stopped");
                self.list.insert(index, job);
                Ok(Foreground::Stopped(notice))
            }
        }
    }

    /// Continue a stopped job in the background (`bg`).
    pub(super) fn background(&mut self, spec: Option<&str>) -> Result<String, String> {
        let index = self.index_of(spec)?;
        let job = &mut self.list[index];
        let Some(group) = job.group.filter(|_| job_control::enabled()) else {
            return Err(format!("%{}: job control is not available here", job.id));
        };
        if !job.stopped {
            return Err(format!("%{}: job already in background", job.id));
        }
        job_control::resume(group);
        job.stopped = false;
        Ok(format!("[{}]+  {} &", job.id, job.text))
    }

    /// Wait for one job, or for all of them, and return the last status. A stopped job
    /// is not waited for: it would never end by itself.
    pub(super) fn wait(&mut self, spec: Option<&str>) -> Result<Option<ExitStatus>, String> {
        if let Some(spec) = spec {
            let index = self.index_of(Some(spec))?;
            if self.list[index].stopped {
                return Err(format!("%{}: job is stopped", self.list[index].id));
            }
            let mut job = self.list.remove(index);
            return Ok(job.wait());
        }
        let mut last = None;
        let mut kept = Vec::new();
        for mut job in std::mem::take(&mut self.list) {
            if job.stopped {
                kept.push(job);
            } else {
                last = job.wait();
            }
        }
        self.list = kept;
        Ok(last)
    }

    /// Stop a job. It is reported as finished at the next notice.
    pub(super) fn kill(&mut self, spec: &str) -> Result<(), String> {
        let index = self.index_of(Some(spec))?;
        self.list[index].kill();
        Ok(())
    }
}

/// What `fg` came to.
#[derive(Debug)]
pub(super) enum Foreground {
    /// The job ended: its status.
    Done(Option<ExitStatus>),
    /// `Ctrl+Z` stopped it again: the notice to show.
    Stopped(String),
}

fn line(job: &Job, current: Option<u32>, label: &str) -> String {
    let marker = if Some(job.id) == current { '+' } else { ' ' };
    format!("[{}]{marker}  {label}  {}", job.id, job.text)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    fn start(script: &str) -> Child {
        Command::new("sh")
            .args(["-c", script])
            .stdin(Stdio::null())
            .spawn()
            .expect("sh should start")
    }

    fn wait_until_over(jobs: &mut Jobs) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let notices = jobs.finished_notices();
            if !notices.is_empty() || Instant::now() > deadline {
                return notices;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn jobs_are_numbered_from_one() {
        let mut jobs = Jobs::default();

        let (first, _) = jobs.add("one", vec![start("sleep 5")], None);
        let (second, _) = jobs.add("two", vec![start("sleep 5")], None);

        assert_eq!((first, second), (1, 2));
        jobs.kill("%1").expect("kill");
        jobs.kill("%2").expect("kill");
        let _ = jobs.wait(None);
    }

    #[test]
    fn a_running_job_is_listed_and_a_finished_one_is_noticed_once() {
        let mut jobs = Jobs::default();
        jobs.add("sleep 5", vec![start("sleep 5")], None);
        jobs.add("quick", vec![start("exit 0")], None);

        let notices = wait_until_over(&mut jobs);

        assert_eq!(notices, ["[2]+  Done  quick"]);
        assert_eq!(jobs.listing(), ["[1]+  Running  sleep 5"]);
        assert!(jobs.finished_notices().is_empty());
        jobs.kill("%1").expect("kill");
        let _ = jobs.wait(None);
    }

    #[test]
    fn a_failing_job_reports_its_exit_code() {
        let mut jobs = Jobs::default();
        jobs.add("fails", vec![start("exit 7")], None);

        assert_eq!(wait_until_over(&mut jobs), ["[1]+  Exit 7  fails"]);
    }

    #[test]
    fn foreground_waits_and_removes_the_job() {
        let mut jobs = Jobs::default();
        jobs.add("slow", vec![start("sleep 0.2; exit 4")], None);

        let mut announced = String::new();
        let Foreground::Done(status) = jobs
            .foreground(None, |text| announced = text.to_string())
            .expect("a job")
        else {
            panic!("the job should have ended");
        };

        assert_eq!(announced, "slow");
        assert_eq!(describe(status).1, 4);
        assert!(jobs.is_empty());
    }

    #[test]
    fn a_job_is_found_by_number_with_or_without_a_percent_sign() {
        let mut jobs = Jobs::default();
        jobs.add("a", vec![start("exit 0")], None);
        jobs.add("b", vec![start("exit 0")], None);

        assert!(jobs.foreground(Some("%1"), |_| {}).is_ok());
        assert!(jobs.foreground(Some("2"), |_| {}).is_ok());
        assert_eq!(
            jobs.foreground(Some("%3"), |_| {}).unwrap_err(),
            "%3: no such job"
        );
        assert_eq!(
            jobs.foreground(Some("x"), |_| {}).unwrap_err(),
            "x: invalid job specification"
        );
        assert_eq!(jobs.foreground(None, |_| {}).unwrap_err(), "no current job");
    }

    #[test]
    fn kill_stops_a_running_job() {
        let mut jobs = Jobs::default();
        jobs.add("sleep 30", vec![start("sleep 30")], None);

        jobs.kill("%1").expect("kill");
        let notices = wait_until_over(&mut jobs);

        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(notices[0].contains("Exit"), "{notices:?}");
    }

    #[test]
    fn wait_without_a_job_waits_for_all_of_them() {
        let mut jobs = Jobs::default();
        jobs.add("a", vec![start("sleep 0.1")], None);
        jobs.add("b", vec![start("sleep 0.1; exit 2")], None);

        let status = jobs.wait(None).expect("wait");

        assert_eq!(describe(status).1, 2);
        assert!(jobs.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_stopped_job_is_listed_continued_and_not_waited_for() {
        job_control::enable_groups_without_terminal();
        let mut jobs = Jobs::default();
        let outcome = job_control::run_foreground(
            {
                let mut command = std::process::Command::new("sh");
                command.args(["-c", "kill -STOP $$; exit 5"]);
                command
            },
            "sh stop",
        )
        .expect("starts");
        assert!(matches!(outcome, job_control::Finished::Stopped));

        let notices = jobs.adopt_stopped();
        assert_eq!(notices, vec!["[1]+  Stopped  sh stop".to_string()]);
        assert!(jobs.has_stopped());
        assert_eq!(jobs.listing(), vec!["[1]+  Stopped  sh stop".to_string()]);
        assert!(jobs.adopt_stopped().is_empty());

        // A stopped job would never end by itself: `wait` does not hang on it.
        assert_eq!(jobs.wait(Some("%1")).unwrap_err(), "%1: job is stopped");
        assert!(jobs.wait(None).expect("nothing to wait for").is_none());
        assert!(jobs.has_stopped());

        // `bg` continues it in the background and it ends by itself.
        assert_eq!(jobs.background(None).unwrap(), "[1]+  sh stop &");
        assert!(!jobs.has_stopped());
        assert_eq!(
            jobs.background(None).unwrap_err(),
            "%1: job already in background"
        );
        let notices = wait_until_over(&mut jobs);
        assert!(notices[0].contains("Exit 5"), "{notices:?}");
    }

    #[cfg(unix)]
    #[test]
    fn fg_continues_a_stopped_job_in_the_foreground() {
        job_control::enable_groups_without_terminal();
        let mut jobs = Jobs::default();
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "kill -STOP $$; exit 6"]);
        let _ = job_control::run_foreground(command, "sh again").expect("starts");
        jobs.adopt_stopped();

        let Foreground::Done(status) = jobs.foreground(None, |_| {}).expect("a job") else {
            panic!("the job should have ended");
        };
        assert_eq!(describe(status).1, 6);
        assert!(jobs.is_empty());
    }
}
