use std::path::PathBuf;
fn queue_target(
    length: usize,
    current: Option<usize>,
    forward: bool,
    repeat: bool,
    shuffle: bool,
    played: &[usize],
    random: usize,
) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let Some(current) = current.filter(|&index| index < length) else {
        return Some(0);
    };
    if shuffle && forward {
        let mut candidates: Vec<_> = (0..length)
            .filter(|&index| index != current && !played.contains(&index))
            .collect();
        if candidates.is_empty() && repeat {
            candidates = (0..length)
                .filter(|&index| length == 1 || index != current)
                .collect();
        }
        return (!candidates.is_empty()).then(|| candidates[random % candidates.len()]);
    }
    if forward {
        if current + 1 < length {
            Some(current + 1)
        } else if repeat {
            Some(0)
        } else {
            None
        }
    } else if current > 0 {
        Some(current - 1)
    } else if repeat {
        Some(length - 1)
    } else {
        None
    }
}

#[derive(Default, Debug)]
pub struct QueueState {
    entries: Vec<(PathBuf, String)>,
    current: Option<usize>,
    played: Vec<usize>,
    pub shuffle: bool,
    pub repeat: bool,
}
impl QueueState {
    pub fn entries(&self) -> &[(PathBuf, String)] {
        &self.entries
    }
    pub fn current(&self) -> Option<usize> {
        self.current
    }
    pub fn enqueue(&mut self, path: PathBuf, title: String) -> usize {
        self.entries.push((path, title));
        self.entries.len() - 1
    }
    pub fn find_or_enqueue(&mut self, path: PathBuf, title: String) -> usize {
        self.entries
            .iter()
            .position(|(p, _)| p == &path)
            .unwrap_or_else(|| self.enqueue(path, title))
    }
    pub fn select(&mut self, index: usize) {
        if index < self.entries.len() {
            self.current = Some(index);
            if !self.played.contains(&index) {
                self.played.push(index);
            }
        }
    }
    pub fn target(&mut self, forward: bool, random: usize) -> Option<usize> {
        let target = queue_target(
            self.entries.len(),
            self.current,
            forward,
            self.repeat,
            self.shuffle,
            &self.played,
            random,
        );
        if self.shuffle && self.played.len() >= self.entries.len() && target.is_some() {
            self.played.clear();
        }
        target
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current = None;
        self.played.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::queue_target;
    #[test]
    fn queue_navigation_handles_empty_end_repeat_and_cleared_cursor() {
        assert_eq!(queue_target(0, None, true, true, true, &[], 0), None);
        assert_eq!(
            queue_target(3, Some(0), true, false, false, &[], 0),
            Some(1)
        );
        assert_eq!(queue_target(3, Some(2), true, false, false, &[], 0), None);
        assert_eq!(queue_target(3, Some(2), true, true, false, &[], 0), Some(0));
        assert_eq!(
            queue_target(3, Some(0), false, true, false, &[], 0),
            Some(2)
        );
        assert_eq!(queue_target(3, None, true, false, false, &[], 0), Some(0));
    }
    #[test]
    fn shuffle_chooses_another_valid_track() {
        for random in 0..20 {
            let next = queue_target(4, Some(2), true, false, true, &[2], random).unwrap();
            assert!(next < 4);
            assert_ne!(next, 2);
        }
        assert_eq!(queue_target(1, Some(0), true, false, true, &[0], 0), None);
        assert_eq!(
            queue_target(3, Some(1), true, false, true, &[0, 1], 0),
            Some(2)
        );
        assert_eq!(
            queue_target(3, Some(2), true, false, true, &[0, 1, 2], 0),
            None
        );
        assert_eq!(
            queue_target(3, Some(2), true, true, true, &[0, 1, 2], 0),
            Some(0)
        );
        assert_eq!(queue_target(1, Some(0), true, true, true, &[0], 0), Some(0));
    }
}
