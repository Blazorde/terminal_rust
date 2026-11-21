//! Background jobs (see ADR-0013): programs started with `&`, and the `jobs`,
//! `fg`, `wait` and `kill` commands that manage them.

use std::process::{Child, ExitStatus};

use crate::commands::external;

/// One background pipeline: its programs and the text that started it.
struct Job {
    id: u32,
    text: String,
    /// The programs of the pipeline.
    children: Vec<Child>,
    /// What each program ended with, once it has.
    finished: Vec<Option<ExitStatus>>,
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
    pub(super) fn add(&mut self, text: &str, children: Vec<Child>) -> (u32, u32) {
        self.next_id += 1;
        let finished = vec![None; children.len()];
        let job = Job {
            id: self.next_id,
            text: text.trim().to_string(),
            children,
            finished,
        };
        let reported = (job.id, job.pid());
        self.list.push(job);
        reported
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

    /// Wait for one job (or the most recent) and return its text and status.
    pub(super) fn foreground(
        &mut self,
        spec: Option<&str>,
    ) -> Result<(String, Option<ExitStatus>), String> {
        let index = self.index_of(spec)?;
        let mut job = self.list.remove(index);
        let status = job.wait();
        Ok((job.text, status))
    }

    /// Wait for one job, or for all of them, and return the last status.
    pub(super) fn wait(&mut self, spec: Option<&str>) -> Result<Option<ExitStatus>, String> {
        if spec.is_some() {
            return self.foreground(spec).map(|(_, status)| status);
        }
        let mut last = None;
        for mut job in std::mem::take(&mut self.list) {
            last = job.wait();
        }
        Ok(last)
    }

    /// Stop a job. It is reported as finished at the next notice.
    pub(super) fn kill(&mut self, spec: &str) -> Result<(), String> {
        let index = self.index_of(Some(spec))?;
        self.list[index].kill();
        Ok(())
    }
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

        let (first, _) = jobs.add("one", vec![start("sleep 5")]);
        let (second, _) = jobs.add("two", vec![start("sleep 5")]);

        assert_eq!((first, second), (1, 2));
        jobs.kill("%1").expect("kill");
        jobs.kill("%2").expect("kill");
        let _ = jobs.wait(None);
    }

    #[test]
    fn a_running_job_is_listed_and_a_finished_one_is_noticed_once() {
        let mut jobs = Jobs::default();
        jobs.add("sleep 5", vec![start("sleep 5")]);
        jobs.add("quick", vec![start("exit 0")]);

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
        jobs.add("fails", vec![start("exit 7")]);

        assert_eq!(wait_until_over(&mut jobs), ["[1]+  Exit 7  fails"]);
    }

    #[test]
    fn foreground_waits_and_removes_the_job() {
        let mut jobs = Jobs::default();
        jobs.add("slow", vec![start("sleep 0.2; exit 4")]);

        let (text, status) = jobs.foreground(None).expect("a job");

        assert_eq!(text, "slow");
        assert_eq!(describe(status).1, 4);
        assert!(jobs.is_empty());
    }

    #[test]
    fn a_job_is_found_by_number_with_or_without_a_percent_sign() {
        let mut jobs = Jobs::default();
        jobs.add("a", vec![start("exit 0")]);
        jobs.add("b", vec![start("exit 0")]);

        assert!(jobs.foreground(Some("%1")).is_ok());
        assert!(jobs.foreground(Some("2")).is_ok());
        assert_eq!(jobs.foreground(Some("%3")).unwrap_err(), "%3: no such job");
        assert_eq!(
            jobs.foreground(Some("x")).unwrap_err(),
            "x: invalid job specification"
        );
        assert_eq!(jobs.foreground(None).unwrap_err(), "no current job");
    }

    #[test]
    fn kill_stops_a_running_job() {
        let mut jobs = Jobs::default();
        jobs.add("sleep 30", vec![start("sleep 30")]);

        jobs.kill("%1").expect("kill");
        let notices = wait_until_over(&mut jobs);

        assert_eq!(notices.len(), 1, "{notices:?}");
        assert!(notices[0].contains("Exit"), "{notices:?}");
    }

    #[test]
    fn wait_without_a_job_waits_for_all_of_them() {
        let mut jobs = Jobs::default();
        jobs.add("a", vec![start("sleep 0.1")]);
        jobs.add("b", vec![start("sleep 0.1; exit 2")]);

        let status = jobs.wait(None).expect("wait");

        assert_eq!(describe(status).1, 2);
        assert!(jobs.is_empty());
    }
}
