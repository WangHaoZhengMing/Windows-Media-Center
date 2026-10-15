use crate::{CATEGORIES, Category, Direction::*, Entry, EntryId};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Each row remembers its own selection, including while a destination is open.
#[derive(Default, Debug)]
pub struct HomeState {
    category: usize,
    selections: [usize; CATEGORIES.len()],
    opened: Option<EntryId>,
}

impl HomeState {
    pub fn category_index(&self) -> usize {
        self.category
    }

    pub fn category(&self) -> &'static Category {
        &CATEGORIES[self.category]
    }

    pub fn selected_index(&self) -> usize {
        self.selections[self.category]
    }

    pub fn selection_for_category(&self, category: usize) -> Option<usize> {
        self.selections.get(category).copied()
    }

    pub fn selected_entry(&self) -> &'static Entry {
        &self.category().entries[self.selected_index()]
    }

    pub fn opened(&self) -> Option<EntryId> {
        self.opened
    }

    pub fn navigate(&mut self, direction: Direction) {
        if self.opened.is_some() {
            return;
        }
        match direction {
            Up => self.category = self.category.saturating_sub(1),
            Down => self.category = (self.category + 1).min(CATEGORIES.len() - 1),
            Left => {
                self.selections[self.category] =
                    previous(self.selected_index(), self.category().entries.len())
            }
            Right => {
                self.selections[self.category] =
                    next(self.selected_index(), self.category().entries.len())
            }
        }
    }

    pub fn select_category(&mut self, index: usize) {
        if index < CATEGORIES.len() && self.opened.is_none() {
            self.category = index;
        }
    }

    pub fn select_entry(&mut self, index: usize) {
        if index < self.category().entries.len() && self.opened.is_none() {
            self.selections[self.category] = index;
        }
    }

    pub fn activate(&mut self) {
        self.opened = Some(self.selected_entry().id);
    }

    pub fn back(&mut self) {
        self.opened = None;
    }
}

fn previous(index: usize, count: usize) -> usize {
    (index + count - 1) % count
}

fn next(index: usize, count: usize) -> usize {
    (index + 1) % count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_categories_preserves_each_rows_selection() {
        let mut state = HomeState::default();
        state.navigate(Right);
        state.navigate(Right);
        state.navigate(Down);
        state.navigate(Right);
        state.navigate(Up);
        assert_eq!(state.selected_entry().id, EntryId::PhotoDates);
        state.navigate(Down);
        assert_eq!(state.selected_entry().id, EntryId::RecentVideos);
    }

    #[test]
    fn returning_from_a_destination_restores_focus() {
        let mut state = HomeState::default();
        state.navigate(Left);
        state.activate();
        assert_eq!(state.opened(), Some(EntryId::Favorites));
        state.navigate(Down);
        state.back();
        assert_eq!(state.category_index(), 0);
        assert_eq!(state.selected_entry().id, EntryId::Favorites);
    }

    #[test]
    fn horizontal_wrapping_respects_rows_with_different_lengths() {
        let mut state = HomeState::default();
        for _ in 0..100 {
            state.navigate(Down);
            state.navigate(Left);
            assert!(state.selected_index() < state.category().entries.len());
        }
        assert_eq!(state.category_index(), CATEGORIES.len() - 1);
    }

    #[test]
    fn vertical_navigation_stops_at_list_boundaries() {
        let mut state = HomeState::default();
        state.navigate(Up);
        assert_eq!(state.category_index(), 0);
        for index in 1..CATEGORIES.len() {
            state.navigate(Down);
            assert_eq!(state.category_index(), index);
        }
        state.navigate(Down);
        assert_eq!(state.category_index(), CATEGORIES.len() - 1);
        for index in (0..CATEGORIES.len() - 1).rev() {
            state.navigate(Up);
            assert_eq!(state.category_index(), index);
        }
    }

    #[test]
    fn stale_pointer_indices_cannot_invalidate_selection() {
        let mut state = HomeState::default();
        state.select_category(usize::MAX);
        state.select_entry(usize::MAX);
        assert_eq!(state.selected_entry().id, EntryId::PhotoLibrary);
    }
}
