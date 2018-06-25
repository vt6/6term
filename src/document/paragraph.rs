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
use pango::{self, LayoutExt};
use pangocairo;

pub struct Paragraph {
    text: String, //This is private to ensure that we notice all write access.
    layout: Option<ParagraphLayout>,
}

impl Paragraph {
    pub fn new(text: String) -> Paragraph {
        Paragraph {
            text: text,
            layout: None,
        }
    }

    pub fn text(&self) -> &String {
        &self.text
    }
    pub fn text_mut(&mut self) -> &mut String {
        self.layout = None;
        &mut self.text
    }

    ///Returns the local height that the paragraph occupies on screen.
    ///FIXME Docs are unclear about whether this is in pixels or something
    ///else, so HiDPI rendering might be broken.
    pub fn prepare_rendering(&mut self, pixel_width: i32, ctx: &pango::Context) -> i32 {
        match self.layout {
            None => {
                let mut layout = ParagraphLayout::new(&self.text, pixel_width, ctx);
                let height = layout.get_logical_extents().height;
                self.layout = Some(layout);
                height
            },
            Some(ref mut layout) => {
                layout.update(pixel_width);
                layout.get_logical_extents().height
            }
        }
    }

    ///The current coordinates of the cairo::Context are at the
    ///upper left corner of the paragraph.
    ///FIXME check with RTL text and RTL locale
    pub fn render(&self, ctx: &cairo::Context) {
        self.layout.as_ref().unwrap().render(ctx);
    }
}

struct ParagraphLayout {
    layout: pango::Layout,
}

impl ParagraphLayout {
    fn new(text: &str, pixel_width: i32, ctx: &pango::Context) -> ParagraphLayout {
        let layout = pango::Layout::new(ctx);
        layout.set_wrap(pango::WrapMode::WordChar);
        layout.set_width(pixel_width * pango::SCALE);
        layout.set_text(&text);
        ParagraphLayout { layout: layout }
    }

    fn update(&mut self, pixel_width: i32) {
        self.layout.set_width(pixel_width * pango::SCALE);
        self.layout.context_changed();
    }

    fn get_logical_extents(&self) -> pango::Rectangle {
        self.layout.get_pixel_extents().1
    }

    fn render(&self, ctx: &cairo::Context) {
        //show_layout requires the cursor to point to the start of the baseline
        let extents = self.get_logical_extents();
        ctx.rel_move_to(-extents.x as f64, -extents.y as f64);
        pangocairo::functions::show_layout(ctx, &self.layout);
        ctx.rel_move_to(extents.x as f64, extents.y as f64);
    }
}
