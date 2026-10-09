use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::git::PromptGitInfo;

struct CacheEntry {
    path: PathBuf,
    value: Option<PromptGitInfo>,
    refreshed_at: Instant,
}

pub struct GitPromptCache {
    entry: Option<CacheEntry>,
    ttl: Duration,
}

impl GitPromptCache {
    pub fn new(ttl: Duration) -> Self {
        Self { entry: None, ttl }
    }

    pub fn get_or_refresh(
        &mut self,
        path: &Path,
        load: impl FnOnce(&Path) -> Option<PromptGitInfo>,
    ) -> Option<PromptGitInfo> {
        self.get_or_refresh_at(path, Instant::now(), load)
    }

    fn get_or_refresh_at(
        &mut self,
        path: &Path,
        now: Instant,
        load: impl FnOnce(&Path) -> Option<PromptGitInfo>,
    ) -> Option<PromptGitInfo> {
        if let Some(entry) = &self.entry
            && entry.path == path
            && now.duration_since(entry.refreshed_at) < self.ttl
        {
            return entry.value.clone();
        }

        let value = load(path);
        self.entry = Some(CacheEntry {
            path: path.to_path_buf(),
            value: value.clone(),
            refreshed_at: now,
        });
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn info(branch: &str) -> PromptGitInfo {
        PromptGitInfo {
            branch: branch.to_string(),
            dirty: false,
        }
    }

    #[test]
    fn reuses_a_value_within_the_ttl() {
        let ttl = Duration::from_millis(250);
        let started = Instant::now();
        let calls = Cell::new(0);
        let mut cache = GitPromptCache::new(ttl);

        let first = cache.get_or_refresh_at(Path::new("repo"), started, |_| {
            calls.set(calls.get() + 1);
            Some(info("main"))
        });
        let second = cache.get_or_refresh_at(
            Path::new("repo"),
            started + Duration::from_millis(100),
            |_| {
                calls.set(calls.get() + 1);
                Some(info("changed"))
            },
        );

        assert_eq!(first, second);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn refreshes_after_the_ttl_or_a_directory_change() {
        let ttl = Duration::from_millis(250);
        let started = Instant::now();
        let calls = Cell::new(0);
        let mut cache = GitPromptCache::new(ttl);

        cache.get_or_refresh_at(Path::new("first"), started, |_| {
            calls.set(calls.get() + 1);
            Some(info("main"))
        });
        cache.get_or_refresh_at(Path::new("first"), started + ttl, |_| {
            calls.set(calls.get() + 1);
            Some(info("main"))
        });
        cache.get_or_refresh_at(Path::new("second"), started + ttl, |_| {
            calls.set(calls.get() + 1);
            None
        });

        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn caches_an_unavailable_git_context() {
        let ttl = Duration::from_secs(1);
        let started = Instant::now();
        let calls = Cell::new(0);
        let mut cache = GitPromptCache::new(ttl);

        for elapsed in [Duration::ZERO, Duration::from_millis(100)] {
            let value = cache.get_or_refresh_at(Path::new("outside"), started + elapsed, |_| {
                calls.set(calls.get() + 1);
                None
            });
            assert!(value.is_none());
        }

        assert_eq!(calls.get(), 1);
    }
}
