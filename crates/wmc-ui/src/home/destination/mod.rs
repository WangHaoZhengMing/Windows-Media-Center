mod about;
mod library;
mod media_locations;
mod queue;
mod settings;
mod shell;

use super::HomeView;
use crate::theme::Layout;
use gpui_kit::{prelude::*, *};
use wmc_core::{DestinationEffect, DestinationKind, Entry};

impl HomeView {
    pub(super) fn destination(
        &self,
        entry: &'static Entry,
        layout: Layout,
        progress: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let page = match self.destination_state.kind {
            DestinationKind::Library(kind) => self.library_page(entry, kind, layout, window, cx),
            DestinationKind::Queue => self.queue_page(layout, cx),
            _ => self.settings_page(layout, cx),
        };
        let menu_alpha = self
            .context_menu
            .as_ref()
            .map_or(1., |menu| menu.pose(cx.reduce_motion()).page_alpha);
        div()
            .absolute()
            .size_full()
            .opacity(progress * menu_alpha)
            .child(
                div()
                    .absolute()
                    .size_full()
                    .top(layout.unit((1. - progress) * 18.))
                    .child(page),
            )
    }

    pub(super) fn activate_destination(&mut self, cx: &mut Context<Self>) {
        if self.destination_state.kind == DestinationKind::Queue && self.destination_state.row == 1
        {
            self.clear_queue(cx);
            return;
        }
        match self.destination_state.activate() {
            DestinationEffect::None => cx.notify(),
            DestinationEffect::Close => self.back(cx),
            DestinationEffect::ApplyAppearance(ambient) => {
                self.ambient_motion = ambient;
                self.back(cx);
            }
        }
    }
}
