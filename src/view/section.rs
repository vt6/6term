/*******************************************************************************
*
* Copyright 2018 Stefan Majewsky <majewsky@gmx.net>
*
* This program is free software: you can redistribute it and/or modify it under
* the terms of the GNU General Public License as published by the Free Software
* Foundation, either version 3 of the License, or (at your option) any later
* version.
*
* This program is distributed in the hope that it will be useful, but WITHOUT ANY
* WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
* A PARTICULAR PURPOSE. See the GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License along with
* this program. If not, see <http://www.gnu.org/licenses/>.
*
*******************************************************************************/

use cairo;
use gtk::{self, WidgetExt};
use pango::{self, LayoutExt};
use pangocairo;

use std::cmp::max;

use model;

///The render state for a model::Section. This is separate from model::Section
///because model::Section needs to implement std::marker::Send, but some things in
///here cannot be moved away from the GUI thread.
pub struct Section {
    layout: pango::Layout,
    ///The last observed value of `model.generation`. When different form
    ///`model.generation`, this means we need to update `self.layout` because the
    ///model has changed.
    layout_generation: u64,
}

impl Section {
    pub fn new(model: &model::Section, canvas: &gtk::DrawingArea) -> Section {
        let layout = canvas.create_pango_layout(Some(model.text())).unwrap();
        layout.set_wrap(pango::WrapMode::WordChar);
        Section {
            layout: layout,
            layout_generation: model.generation(),
        }
    }

    ///Returns the local height that the section occupies on screen.
    ///FIXME Docs are unclear about whether this is in pixels or something
    ///else, so HiDPI rendering might be broken.
    pub fn prepare_rendering(&mut self, model: &model::Section, pixel_width: i32) -> i32 {
        self.layout.set_width(pixel_width * pango::SCALE);
        if self.layout_generation != model.generation() {
            self.layout.set_text(&model.text());
            self.layout_generation = model.generation();
        }

        //Pango context may have been attached to a new Cairo context
        self.layout.context_changed();
        //return logical height
        self.get_logical_extents().height
    }

    ///The current coordinates of the cairo::Context must be at the
    ///upper left corner of the section.
    ///FIXME check with RTL text and RTL locale
    pub fn render(&self, model: &model::Section, ctx: &cairo::Context, show_cursor: bool) {
        ctx.save();

        //show_layout requires the cursor to point to the start of the baseline
        let extents = self.get_logical_extents();
        ctx.move_to(-extents.x as f64, -extents.y as f64);
        pangocairo::functions::show_layout(ctx, &self.layout);

        if show_cursor {
            let (cursor_rect, _) = self.layout.get_cursor_pos(model.input_cursor() as i32);
            ctx.rectangle(
                rescale_p2c(cursor_rect.x),
                rescale_p2c(cursor_rect.y),
                rescale_p2c(max(cursor_rect.width, 1 * pango::SCALE)),
                rescale_p2c(cursor_rect.height),
            );
            ctx.fill();
        }

        ctx.restore();
    }

    fn get_logical_extents(&self) -> pango::Rectangle {
        self.layout.get_pixel_extents().1
    }
}

fn rescale_p2c(pango_dimension: i32) -> f64 {
    (pango_dimension as f64) / (pango::SCALE as f64)
}
