//! How a typed prefix is compared with a name. Every source is first asked for
//! the names in the case typed, and only without any for those in another case.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Case {
    Exact,
    Ignored,
}

/// The order in which the cases are tried.
pub(super) const CASES: [Case; 2] = [Case::Exact, Case::Ignored];

impl Case {
    pub(super) fn starts_with(self, name: &str, prefix: &str) -> bool {
        match self {
            Self::Exact => name.starts_with(prefix),
            Self::Ignored => {
                let mut name = name.chars().flat_map(char::to_lowercase);
                prefix
                    .chars()
                    .flat_map(char::to_lowercase)
                    .all(|wanted| name.next() == Some(wanted))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_case_counts_only_when_exact() {
        assert!(Case::Exact.starts_with("README", "READ"));
        assert!(!Case::Exact.starts_with("README", "read"));
        assert!(Case::Ignored.starts_with("README", "read"));
        assert!(Case::Ignored.starts_with("\u{c9}t\u{e9}", "\u{e9}T"));
        assert!(!Case::Ignored.starts_with("read", "readme"));
        assert!(!Case::Ignored.starts_with("README", "x"));
    }
}
