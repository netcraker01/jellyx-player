//! Queue controller — pure queue management logic for playback.
//!
//! Operates on `QueueState` without any platform dependencies. The desktop
//! and TUI frontends use this controller to manage queue mutations, then
//! delegate the actual audio playback to their respective audio backends.

use rand::seq::SliceRandom;

use crate::playback_models::{QueueState, RepeatMode};

/// Pure queue controller — no I/O, no audio, no platform dependencies.
///
/// All methods take `&mut QueueState` and return the resulting state changes.
/// Callers are responsible for emitting events and triggering audio playback.
pub struct QueueController;

impl QueueController {
    /// Compute the next track index, applying shuffle and repeat modes.
    pub fn compute_next_index(queue: &mut QueueState) -> Option<usize> {
        if queue.tracks.is_empty() {
            return None;
        }
        if queue.shuffle {
            Self::shuffle_next_track(queue)
        } else {
            Self::sequential_next_index(
                queue.current_index.unwrap_or(0),
                queue.tracks.len(),
                queue.repeat_mode,
            )
        }
    }

    /// Pick the next sequential index, applying repeat-all and repeat-one.
    pub fn sequential_next_index(current: usize, len: usize, repeat: RepeatMode) -> Option<usize> {
        match repeat {
            RepeatMode::One => Some(current),
            _ => {
                let candidate = current + 1;
                if candidate < len {
                    Some(candidate)
                } else if repeat == RepeatMode::All {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }

    /// Pick the next shuffle index, avoiding recently played tracks.
    pub fn shuffle_next_track(queue: &mut QueueState) -> Option<usize> {
        let len = queue.tracks.len();
        if len == 0 {
            return None;
        }
        let current = queue.current_index.unwrap_or(0);
        let unplayed: Vec<usize> = (0..len)
            .filter(|i| *i != current && !queue.played_indices.contains(i))
            .collect();
        if unplayed.is_empty() {
            if queue.repeat_mode == RepeatMode::All {
                queue.played_indices.clear();
                queue.played_indices.push(current);
                let next = (0..len)
                    .filter(|i| *i != current)
                    .collect::<Vec<_>>()
                    .choose(&mut rand::thread_rng())
                    .copied();
                if let Some(idx) = next {
                    queue.played_indices.push(idx);
                }
                return next;
            }
            return None;
        }
        let next = unplayed.choose(&mut rand::thread_rng()).copied();
        if let Some(idx) = next {
            queue.played_indices.push(idx);
        }
        next
    }

    /// Compute the previous index (ignores shuffle, respects repeat-all).
    pub fn compute_previous_index(current: usize, len: usize, repeat: RepeatMode) -> usize {
        if current == 0 {
            if repeat == RepeatMode::All {
                len.saturating_sub(1)
            } else {
                0
            }
        } else {
            current - 1
        }
    }

    /// Rebase played indices after a queue item at `removed_index` is removed.
    pub fn rebase_played_indices(played_indices: &mut Vec<usize>, removed_index: usize) {
        played_indices.retain(|&i| i != removed_index);
        for i in played_indices.iter_mut() {
            if *i > removed_index {
                *i -= 1;
            }
        }
    }

    /// Add a track to the end of the queue.
    pub fn add_to_queue(queue: &mut QueueState, track: jellyx_core::models::track::Track) {
        queue.tracks.push(track);
    }

    /// Replace the entire queue with the given tracks and set current to 0.
    pub fn replace_queue(queue: &mut QueueState, tracks: Vec<jellyx_core::models::track::Track>) {
        queue.tracks = tracks;
        queue.current_index = if queue.tracks.is_empty() {
            None
        } else {
            Some(0)
        };
        queue.played_indices.clear();
    }

    /// Remove a track by ID. Returns `(removed_index, was_current, was_before_current)`.
    pub fn remove_from_queue(
        queue: &mut QueueState,
        track_id: &str,
    ) -> Option<(usize, bool, bool)> {
        let position = queue.tracks.iter().position(|t| t.id == track_id)?;
        let removed_index = position;
        let was_current = queue.current_index == Some(removed_index);
        let was_before_current = queue
            .current_index
            .map(|ci| removed_index < ci)
            .unwrap_or(false);

        queue.tracks.remove(removed_index);
        Self::rebase_played_indices(&mut queue.played_indices, removed_index);

        if queue.tracks.is_empty() || was_current {
            queue.current_index = None;
        } else if was_before_current {
            queue.current_index = queue.current_index.map(|ci| ci.saturating_sub(1));
        }

        Some((removed_index, was_current, was_before_current))
    }

    /// Clear the entire queue.
    pub fn clear_queue(queue: &mut QueueState) {
        queue.tracks.clear();
        queue.current_index = None;
        queue.played_indices.clear();
    }

    /// Insert a track immediately after the current position.
    /// Replaces any prior `__play_next__` insertion at the target slot.
    pub fn play_next(queue: &mut QueueState, track: jellyx_core::models::track::Track) {
        let insert_index = match queue.current_index {
            Some(ci) => {
                let target = ci + 1;
                if target < queue.tracks.len()
                    && queue.tracks[target].id.starts_with("__play_next__")
                {
                    queue.tracks.remove(target);
                    Self::rebase_played_indices(&mut queue.played_indices, target);
                }
                target.min(queue.tracks.len())
            }
            None => queue.tracks.len(),
        };
        queue.tracks.insert(insert_index, track);
        queue.current_index = Some(insert_index);
    }

    /// Set shuffle mode.
    pub fn set_shuffle(queue: &mut QueueState, enabled: bool) {
        queue.shuffle = enabled;
    }

    /// Set repeat mode.
    pub fn set_repeat(queue: &mut QueueState, mode: RepeatMode) {
        queue.repeat_mode = mode;
    }

    /// Cycle repeat: Off -> All -> One -> Off.
    pub fn cycle_repeat(queue: &mut QueueState) -> RepeatMode {
        queue.repeat_mode = match queue.repeat_mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        queue.repeat_mode
    }

    /// Get the current track, if any.
    pub fn current_track(queue: &QueueState) -> Option<&jellyx_core::models::track::Track> {
        queue.current_index.and_then(|i| queue.tracks.get(i))
    }

    /// Peek at the next track without advancing.
    pub fn peek_next(queue: &mut QueueState) -> Option<jellyx_core::models::track::Track> {
        let next_index = Self::compute_next_index(queue)?;
        queue.tracks.get(next_index).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jellyx_core::models::source::Source;

    fn test_track(id: &str) -> jellyx_core::models::track::Track {
        jellyx_core::models::track::Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: format!("local-{}", id),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: None,
            duration: None,
            thumbnail: None,
            stream_url: None,
            local_path: None,
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn sequential_next_wraps_with_repeat_all() {
        assert_eq!(
            QueueController::sequential_next_index(2, 3, RepeatMode::All),
            Some(0)
        );
        assert_eq!(
            QueueController::sequential_next_index(0, 3, RepeatMode::Off),
            Some(1)
        );
        assert_eq!(
            QueueController::sequential_next_index(2, 3, RepeatMode::Off),
            None
        );
        assert_eq!(
            QueueController::sequential_next_index(1, 3, RepeatMode::One),
            Some(1)
        );
    }

    #[test]
    fn compute_previous_wraps_with_repeat_all() {
        assert_eq!(
            QueueController::compute_previous_index(0, 3, RepeatMode::All),
            2
        );
        assert_eq!(
            QueueController::compute_previous_index(0, 3, RepeatMode::Off),
            0
        );
        assert_eq!(
            QueueController::compute_previous_index(2, 3, RepeatMode::Off),
            1
        );
    }

    #[test]
    fn add_and_replace_queue() {
        let mut q = QueueState::default();
        QueueController::add_to_queue(&mut q, test_track("t1"));
        QueueController::add_to_queue(&mut q, test_track("t2"));
        assert_eq!(q.tracks.len(), 2);

        QueueController::replace_queue(
            &mut q,
            vec![test_track("a"), test_track("b"), test_track("c")],
        );
        assert_eq!(q.tracks.len(), 3);
        assert_eq!(q.current_index, Some(0));
    }

    #[test]
    fn remove_from_queue_adjusts_current() {
        let mut q = QueueState::default();
        QueueController::replace_queue(
            &mut q,
            vec![test_track("a"), test_track("b"), test_track("c")],
        );
        q.current_index = Some(2);

        let result = QueueController::remove_from_queue(&mut q, "b");
        assert!(result.is_some());
        assert_eq!(q.current_index, Some(1));
        assert_eq!(q.tracks.len(), 2);
    }

    #[test]
    fn remove_current_clears_index() {
        let mut q = QueueState::default();
        QueueController::replace_queue(&mut q, vec![test_track("a"), test_track("b")]);
        q.current_index = Some(0);

        let (_, was_current, _) = QueueController::remove_from_queue(&mut q, "a").unwrap();
        assert!(was_current);
        assert_eq!(q.current_index, None);
    }

    #[test]
    fn play_next_inserts_after_current() {
        let mut q = QueueState::default();
        QueueController::replace_queue(&mut q, vec![test_track("a"), test_track("b")]);
        q.current_index = Some(0);

        QueueController::play_next(&mut q, test_track("c"));
        assert_eq!(q.tracks[1].id, "c");
        assert_eq!(q.current_index, Some(1));
    }

    #[test]
    fn cycle_repeat_rotates() {
        let mut q = QueueState::default();
        assert_eq!(QueueController::cycle_repeat(&mut q), RepeatMode::All);
        assert_eq!(QueueController::cycle_repeat(&mut q), RepeatMode::One);
        assert_eq!(QueueController::cycle_repeat(&mut q), RepeatMode::Off);
    }

    #[test]
    fn clear_queue_resets_all() {
        let mut q = QueueState::default();
        QueueController::replace_queue(&mut q, vec![test_track("a")]);
        QueueController::clear_queue(&mut q);
        assert!(q.tracks.is_empty());
        assert_eq!(q.current_index, None);
    }

    #[test]
    fn current_track_returns_none_for_empty() {
        let q = QueueState::default();
        assert!(QueueController::current_track(&q).is_none());
    }

    #[test]
    fn rebase_played_indices_shifts() {
        let mut played = vec![0, 2, 4];
        QueueController::rebase_played_indices(&mut played, 2);
        assert_eq!(played, vec![0, 3]);
    }
}
