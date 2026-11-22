//! Job control: `Ctrl+Z`, `fg` and `bg` (see ADR-0024).
//!
//! A program that runs in the foreground leads a process group of its own, and the
//! terminal is handed to that group while the shell waits for it. The terminal then
//! sends `Ctrl+C` and `Ctrl+Z` to the program and not to the shell, and the shell can
//! tell that the program was *stopped* (the wait asks for it) instead of waiting for
//! ever. The shell takes the terminal back, remembers the group as a stopped job, and
//! `fg` or `bg` continue it later.
//!
//! Everything goes through `rustix` and `nix`, whose process, terminal and signal-mask
//! calls are safe functions, so the program still has no `unsafe` code. It only does
//! anything when the shell runs on a terminal; elsewhere (a pipe, a script, Windows)
//! programs are started and waited for as before.

use std::cell::RefCell;
use std::io;
use std::process::{Child, Command, ExitStatus};

#[cfg(unix)]
pub(crate) type Modes = rustix::termios::Termios;
#[cfg(not(unix))]
pub(crate) type Modes = ();

/// The status a command that was stopped reports: 128 plus the number of `SIGTSTP`.
pub(crate) const STOPPED_STATUS: i32 = 148;

/// What a job that comes back to the foreground needs.
pub(crate) struct Resume<'a> {
    /// How the program left the terminal, put back first.
    pub(crate) modes: Option<&'a Modes>,
}

/// A group of programs that `Ctrl+Z` stopped, with what is needed to continue it.
pub(crate) struct Stopped {
    pub(crate) children: Vec<Child>,
    /// The process group, which is the pid of its first program.
    pub(crate) group: i32,
    /// How the program left the terminal, put back before it continues.
    pub(crate) modes: Option<Modes>,
}

/// How waiting for a foreground group ended.
pub(crate) enum Waited {
    /// Every program is over, with its status, in the order of the group.
    Done(Vec<ExitStatus>),
    /// The group was stopped: it can be continued.
    Stopped(Stopped),
}

/// How a single foreground program ended.
pub(crate) enum Finished {
    Exited(ExitStatus),
    /// The program was stopped and is now a job (see [`take_stopped`]).
    Stopped,
}

thread_local! {
    /// The groups stopped since the last call of [`take_stopped`], with the text of
    /// their command, for the job table to adopt.
    static STOPPED: RefCell<Vec<(String, Stopped)>> = const { RefCell::new(Vec::new()) };
}

/// Set a stopped group aside for the job table, which adopts it before the next prompt.
pub(crate) fn set_aside(stopped: Stopped, text: &str) {
    STOPPED.with(|list| list.borrow_mut().push((text.to_string(), stopped)));
}

/// The groups stopped since the last call, with the text of their command.
pub(crate) fn take_stopped() -> Vec<(String, Stopped)> {
    STOPPED.with(|list| std::mem::take(&mut *list.borrow_mut()))
}

/// Start `command` in the foreground and wait for it. `text` names the job if it is
/// stopped. An error when the program could not start.
pub(crate) fn run_foreground(mut command: Command, text: &str) -> io::Result<Finished> {
    prepare(&mut command, None);
    let child = command.spawn()?;
    join_group(&child, None, true);
    match wait_foreground(vec![child], None) {
        Waited::Done(mut statuses) => Ok(Finished::Exited(
            statuses.pop().unwrap_or_else(success_status),
        )),
        Waited::Stopped(stopped) => {
            set_aside(stopped, text);
            Ok(Finished::Stopped)
        }
    }
}

#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(not(unix))]
pub(crate) use other::*;

#[cfg(unix)]
mod unix {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Child, Command, ExitStatus};
    use std::sync::atomic::{AtomicBool, Ordering};

    use nix::sys::signal::{SigSet, SigmaskHow, Signal, pthread_sigmask};
    use rustix::process::{Pid, WaitOptions, WaitStatus, getpgrp, kill_process_group, waitpid};
    use rustix::termios::{OptionalActions, tcgetattr, tcgetpgrp, tcsetattr, tcsetpgrp};

    use super::{Modes, Resume, Stopped, Waited};

    /// Programs lead their own process group and are waited for as a group.
    static GROUPS: AtomicBool = AtomicBool::new(false);
    /// The terminal is handed to the group of the foreground program.
    static TERMINAL: AtomicBool = AtomicBool::new(false);

    /// Turn job control on when the shell is on a terminal and owns it. Returns
    /// whether it is on. The shell catches `SIGTSTP`, so that `Ctrl+Z` pressed while
    /// the shell has the terminal does not stop it (a caught signal is reset to its
    /// default in the programs, an ignored one would not be).
    pub(crate) fn init() -> bool {
        let terminal = std::io::stdin();
        let owns_terminal = matches!(
            tcgetpgrp(&terminal),
            Ok(foreground) if foreground == getpgrp()
        );
        if !owns_terminal {
            return false;
        }
        let ignored = std::sync::Arc::new(AtomicBool::new(false));
        if signal_hook::flag::register(signal_hook::consts::SIGTSTP, ignored).is_err() {
            return false;
        }
        GROUPS.store(true, Ordering::SeqCst);
        TERMINAL.store(true, Ordering::SeqCst);
        true
    }

    /// Whether programs run as job-controlled groups.
    pub(crate) fn enabled() -> bool {
        GROUPS.load(Ordering::SeqCst)
    }

    /// Process groups without a terminal, for tests that have none.
    #[cfg(test)]
    pub(crate) fn enable_groups_without_terminal() {
        GROUPS.store(true, Ordering::SeqCst);
    }

    /// Make `command` lead a process group (`group` is `None`), or join the group
    /// of the first program of its pipeline.
    pub(crate) fn prepare(command: &mut Command, group: Option<i32>) {
        if enabled() {
            command.process_group(group.unwrap_or(0));
        }
    }

    fn pid(raw: u32) -> Option<Pid> {
        Pid::from_raw(i32::try_from(raw).ok()?)
    }

    /// Put the child in its group from the shell's side too, so that the group exists
    /// whichever of the two runs first. A foreground program gets the terminal at
    /// once: it may read it before the shell has finished starting its pipeline.
    pub(crate) fn join_group(child: &Child, group: Option<i32>, foreground: bool) {
        if !enabled() {
            return;
        }
        let Some(member) = pid(child.id()) else {
            return;
        };
        let leader = group.and_then(Pid::from_raw).unwrap_or(member);
        let _ = rustix::process::setpgid(Some(member), Some(leader));
        if foreground && group.is_none() {
            give_terminal(leader.as_raw_nonzero().get());
        }
    }

    /// The group a program started with [`prepare`] leads.
    pub(crate) fn group_of(child: &Child) -> i32 {
        pid(child.id()).map_or(0, |group| group.as_raw_nonzero().get())
    }

    /// Run `action` with the signals blocked that the terminal sends to a process that
    /// is not in its foreground group (`SIGTTOU`, `SIGTTIN`): the shell is in the
    /// background while a program has the terminal, and it must still be able to take
    /// the terminal back and set its modes without being stopped. Only here, and not
    /// for good, because programs would inherit the blocked signals.
    fn without_terminal_signals(action: impl FnOnce()) {
        let mut blocked = SigSet::empty();
        blocked.add(Signal::SIGTTOU);
        blocked.add(Signal::SIGTTIN);
        let mut before = SigSet::empty();
        let blocking = pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&blocked), Some(&mut before));
        action();
        if blocking.is_ok() {
            let _ = pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&before), None);
        }
    }

    /// Give the terminal to `group`, which has to be in the foreground.
    fn give_terminal(group: i32) {
        if TERMINAL.load(Ordering::SeqCst)
            && let Some(group) = Pid::from_raw(group)
        {
            without_terminal_signals(|| {
                let _ = tcsetpgrp(std::io::stdin(), group);
            });
        }
    }

    /// Take the terminal back for the shell.
    fn take_terminal() {
        if TERMINAL.load(Ordering::SeqCst) {
            without_terminal_signals(|| {
                let _ = tcsetpgrp(std::io::stdin(), getpgrp());
            });
        }
    }

    fn current_modes() -> Option<Modes> {
        if TERMINAL.load(Ordering::SeqCst) {
            tcgetattr(std::io::stdin()).ok()
        } else {
            None
        }
    }

    fn restore_modes(modes: &Modes) {
        if TERMINAL.load(Ordering::SeqCst) {
            without_terminal_signals(|| {
                let _ = tcsetattr(std::io::stdin(), OptionalActions::Drain, modes);
            });
        }
    }

    pub(crate) fn success_status() -> ExitStatus {
        ExitStatus::from_raw(0)
    }

    enum Outcome {
        Over(ExitStatus),
        Stopped,
    }

    /// Wait for one program, and notice when it is stopped.
    fn wait_one(child: &Child) -> Outcome {
        let Some(target) = pid(child.id()) else {
            return Outcome::Over(success_status());
        };
        let mut early_terminal_stops = 0;
        loop {
            match waitpid(Some(target), WaitOptions::UNTRACED) {
                // A program that read or wrote the terminal in the instant before it was
                // handed over was stopped for it: it is in the foreground now, so go on.
                Ok(Some((_, status)))
                    if status.stopped()
                        && TERMINAL.load(Ordering::SeqCst)
                        && early_terminal_stops < 3
                        && is_terminal_stop(&status) =>
                {
                    early_terminal_stops += 1;
                    resume(target.as_raw_nonzero().get());
                }
                Ok(Some((_, status))) => return classify(status),
                Ok(None) => {}
                Err(rustix::io::Errno::INTR) => {}
                // Already collected, or not ours: nothing left to wait for.
                Err(_) => return Outcome::Over(success_status()),
            }
        }
    }

    /// Whether a stopped program was stopped for the terminal (`SIGTTIN`, `SIGTTOU`).
    fn is_terminal_stop(status: &WaitStatus) -> bool {
        status.stopping_signal().is_some_and(|signal| {
            signal == Signal::SIGTTIN as i32 || signal == Signal::SIGTTOU as i32
        })
    }

    fn classify(status: WaitStatus) -> Outcome {
        if status.stopped() {
            Outcome::Stopped
        } else {
            Outcome::Over(ExitStatus::from_raw(status.as_raw()))
        }
    }

    /// Wait for the programs of a foreground group. The terminal goes to the group
    /// while it runs and comes back to the shell after. `resume_with` is given for a
    /// job that comes back to the foreground, which is continued once it has the terminal.
    pub(crate) fn wait_foreground(mut children: Vec<Child>, resume_with: Option<Resume>) -> Waited {
        if !enabled() || children.is_empty() {
            let statuses = children
                .iter_mut()
                .map(|child| child.wait().unwrap_or_else(|_| success_status()))
                .collect();
            return Waited::Done(statuses);
        }
        let group = group_of(&children[0]);
        give_terminal(group);
        if let Some(resume_with) = resume_with {
            // A job that comes back to the foreground: its terminal modes, then `SIGCONT`.
            if let Some(modes) = resume_with.modes {
                restore_modes(modes);
            }
            resume(group);
        }
        let mut statuses = Vec::with_capacity(children.len());
        for child in &children {
            match wait_one(child) {
                Outcome::Over(status) => statuses.push(status),
                Outcome::Stopped => {
                    let modes = current_modes();
                    take_terminal();
                    return Waited::Stopped(Stopped {
                        children,
                        group,
                        modes,
                    });
                }
            }
        }
        take_terminal();
        // The terminal sends `Ctrl+C` to the program, not to the shell: tell the loops
        // of the shell that the user asked to stop.
        if statuses
            .iter()
            .any(|status| status.signal() == Some(Signal::SIGINT as i32))
        {
            crate::commands::external::request_interrupt();
        }
        Waited::Done(statuses)
    }

    /// Continue a stopped group (`SIGCONT`).
    pub(crate) fn resume(group: i32) {
        if let Some(group) = Pid::from_raw(group) {
            let _ = kill_process_group(group, rustix::process::Signal::CONT);
        }
    }
}

#[cfg(not(unix))]
mod other {
    use std::os::windows::process::ExitStatusExt;
    use std::process::{Child, Command, ExitStatus};

    use super::{Modes, Resume, Waited};

    pub(crate) fn init() -> bool {
        false
    }

    pub(crate) fn enabled() -> bool {
        false
    }

    pub(crate) fn prepare(_command: &mut Command, _group: Option<i32>) {}

    pub(crate) fn join_group(_child: &Child, _group: Option<i32>, _foreground: bool) {}

    pub(crate) fn group_of(_child: &Child) -> i32 {
        0
    }

    pub(crate) fn success_status() -> ExitStatus {
        ExitStatus::from_raw(0)
    }

    pub(crate) fn wait_foreground(mut children: Vec<Child>, _resume: Option<Resume>) -> Waited {
        Waited::Done(
            children
                .iter_mut()
                .map(|child| child.wait().unwrap_or_else(|_| success_status()))
                .collect(),
        )
    }

    pub(crate) fn resume(_group: i32) {}
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn a_stopped_program_is_set_aside_and_continues_where_it_was() {
        enable_groups_without_terminal();

        // The program stops itself, as `Ctrl+Z` would stop it.
        let outcome = run_foreground(shell("kill -STOP $$; exit 7"), "sh stop").expect("starts");
        assert!(matches!(outcome, Finished::Stopped));

        let mut stopped = take_stopped();
        assert_eq!(stopped.len(), 1);
        assert!(take_stopped().is_empty(), "the list is taken once");
        let (text, stopped) = stopped.remove(0);
        assert_eq!(text, "sh stop");
        assert_eq!(stopped.children.len(), 1);
        assert_eq!(stopped.group, group_of(&stopped.children[0]));

        // Continued in the foreground, it finishes with its own status.
        match wait_foreground(stopped.children, Some(Resume { modes: None })) {
            Waited::Done(statuses) => assert_eq!(statuses[0].code(), Some(7)),
            Waited::Stopped(_) => panic!("the program should have ended"),
        }
    }

    #[test]
    fn a_pipeline_group_is_stopped_and_continued_as_a_whole() {
        enable_groups_without_terminal();

        let mut first = shell("kill -STOP $$; echo one");
        prepare(&mut first, None);
        first.stdout(std::process::Stdio::piped());
        let mut first = first.spawn().expect("starts");
        join_group(&first, None, false);
        let group = group_of(&first);
        let mut second = shell("cat >/dev/null; exit 3");
        prepare(&mut second, Some(group));
        second.stdin(std::process::Stdio::from(
            first.stdout.take().expect("piped output"),
        ));
        let second = second.spawn().expect("starts");
        join_group(&second, Some(group), false);

        let stopped = match wait_foreground(vec![first, second], None) {
            Waited::Stopped(stopped) => stopped,
            Waited::Done(_) => panic!("the first program stops itself"),
        };
        assert_eq!(stopped.children.len(), 2);

        match wait_foreground(stopped.children, Some(Resume { modes: None })) {
            Waited::Done(statuses) => {
                assert_eq!(statuses.len(), 2);
                assert_eq!(statuses[0].code(), Some(0));
                assert_eq!(statuses[1].code(), Some(3));
            }
            Waited::Stopped(_) => panic!("the pipeline should have ended"),
        }
    }

    #[test]
    fn a_program_killed_by_a_signal_reports_it() {
        enable_groups_without_terminal();

        let outcome = run_foreground(shell("kill -TERM $$"), "sh term").expect("starts");
        match outcome {
            Finished::Exited(status) => assert_eq!(status.signal(), Some(15)),
            Finished::Stopped => panic!("not stopped"),
        }
    }
}
