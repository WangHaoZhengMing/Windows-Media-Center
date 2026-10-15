//! Vocal interval selection and stable display ordering.
use super::{Line, Lyrics};

impl Lyrics {
    /// Last started line, also across instrumental gaps; None before the first.
    pub fn active_line(&self, seconds: f64) -> Option<usize> {
        // Keep an unfinished lead anchored when another voice joins it.
        self.lines
            .iter()
            .enumerate()
            .find(|(_, line)| {
                !line.background && line.vocal_start() <= seconds && seconds < line.vocal_end()
            })
            .map(|(index, _)| index)
            .or_else(|| {
                self.lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| !line.background && line.start <= seconds)
                    .max_by(|a, b| a.1.start.total_cmp(&b.1.start))
                    .map(|(index, _)| index)
            })
    }

    /// At most `limit` leads, plus independently timed background vocals.
    /// Blank instrumental lines are anchors, but are never highlighted text.
    pub fn selected_lines(&self, seconds: f64, limit: usize, tail: f64) -> Vec<usize> {
        let eligible = |line: &Line| {
            seconds >= line.vocal_start()
                && seconds < line.end.min(line.vocal_end() + tail.max(0.))
                && line.words.iter().any(|word| !word.text.trim().is_empty())
        };
        let mut leads: Vec<_> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.background && eligible(line))
            .collect();
        // Actual vocals take priority over the finishing tail.
        leads.sort_by_key(|(index, line)| (seconds >= line.vocal_end(), *index));
        let mut selected: Vec<_> = leads
            .into_iter()
            .take(limit)
            .map(|(index, _)| index)
            .collect();
        // Background vocals have their own timing and do not consume lead slots.
        selected.extend(
            self.lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.background && eligible(line))
                .map(|(index, _)| index),
        );
        selected.sort_unstable();
        selected
    }

    pub fn display_order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.lines.len());
        for (index, line) in self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.background)
        {
            order.push(index);
            if let Some(group) = line.group {
                order.extend(
                    self.lines
                        .iter()
                        .enumerate()
                        .filter(|(_, child)| child.background && child.group == Some(group))
                        .map(|(index, _)| index),
                );
            }
        }
        order.extend(
            self.lines
                .iter()
                .enumerate()
                .filter(|(index, line)| {
                    line.background
                        && !order_contains_parent(&self.lines, line.group)
                        && !order.contains(index)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>(),
        );
        order
    }
}

fn order_contains_parent(lines: &[Line], group: Option<usize>) -> bool {
    group.is_some()
        && lines
            .iter()
            .any(|line| !line.background && line.group == group)
}
