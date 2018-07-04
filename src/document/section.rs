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
use gtk;
use pango::{self, LayoutExt};
use pangocairo;

use std::cmp::max;

pub enum CursorAction {
    Insert(String),
    //TODO replace "Char" by "GraphemeCluster" or sth like that
    DeletePreviousChar, //Backspace key
    DeleteNextChar,     //Delete key
    GotoPreviousChar,   //Left arrow key
    GotoNextChar,       //Right arrow key
}

///A section is some amount of text that appears on screen, starting at the
///beginning of a line and ending at the end of a line.
pub struct Section {
    text: String, //This is private to ensure that we notice all write access.
    text_changed: bool,
    layout: pango::Layout,
    cursor: usize,
}

impl Section {
    pub fn new<W: gtk::WidgetExt>(widget: &W, text: String) -> Section {
        let layout = widget.create_pango_layout(Some(text.as_str())).unwrap();
        layout.set_wrap(pango::WrapMode::WordChar);

        let len = text.len();
        Section {
            text: text,
            text_changed: false,
            layout: layout,
            cursor: len,
        }
    }

    ///Returns whether self.text has changed.
    pub fn execute_cursor_action(&mut self, action: CursorAction) -> bool {
        use self::CursorAction::*;
        match action {
            Insert(ref text) => {
                self.text.insert_str(self.cursor, text);
                self.cursor = self.cursor + text.len();
            },
            DeletePreviousChar | GotoPreviousChar => {
                if self.cursor == 0 { return false; }
                //search for start of previous char
                self.cursor -= 1;
                while !self.text.is_char_boundary(self.cursor) {
                    self.cursor -= 1;
                }
                if let DeletePreviousChar = action {
                    self.text.remove(self.cursor);
                }
            },
            DeleteNextChar => {
                if self.cursor == self.text.len() { return false; }
                self.text.remove(self.cursor); //cursor does not move
            },
            GotoNextChar => {
                if self.cursor == self.text.len() { return false; }
                self.cursor += 1;
                while !self.text.is_char_boundary(self.cursor) {
                    self.cursor += 1;
                }
            },
        }

        //text was changed
        self.text_changed = true;
        true
    }

    ///Returns the local height that the section occupies on screen.
    ///FIXME Docs are unclear about whether this is in pixels or something
    ///else, so HiDPI rendering might be broken.
    pub fn prepare_rendering(&mut self, pixel_width: i32) -> i32 {
        self.layout.set_width(pixel_width * pango::SCALE);
        if self.text_changed {
            self.layout.set_text(&self.text);
            self.text_changed = false;
        }

        //Pango context may have been attached to a new Cairo context
        self.layout.context_changed();
        //return logical height
        self.get_logical_extents().height
    }

    ///The current coordinates of the cairo::Context must be at the
    ///upper left corner of the section.
    ///FIXME check with RTL text and RTL locale
    pub fn render(&self, ctx: &cairo::Context, show_cursor: bool) {
        ctx.save();

        //show_layout requires the cursor to point to the start of the baseline
        let extents = self.get_logical_extents();
        ctx.move_to(-extents.x as f64, -extents.y as f64);
        pangocairo::functions::show_layout(ctx, &self.layout);

        if show_cursor {
            let (cursor_rect, _) = self.layout.get_cursor_pos(self.cursor as i32);
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
