use std::fs::Metadata;
use std::time::UNIX_EPOCH;

use crate::commands::ls::item::LsItem;
use crate::commands::ls::options::{LsOptions, SortMode};

pub fn sort_items(items: &mut [LsItem], options: &LsOptions) {
    match options.sort_mode {
        SortMode::Name => items.sort_unstable_by(|left, right| left.name.cmp(&right.name)),
        SortMode::Size => items.sort_unstable_by(|left, right| {
            right
                .metadata
                .len()
                .cmp(&left.metadata.len())
                .then_with(|| left.name.cmp(&right.name))
        }),
        SortMode::Time => items.sort_unstable_by(|left, right| {
            modified_timestamp(&right.metadata)
                .cmp(&modified_timestamp(&left.metadata))
                .then_with(|| left.name.cmp(&right.name))
        }),
    }

    if options.reverse {
        items.reverse();
    }
}

fn modified_timestamp(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
