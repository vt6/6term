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

use gtk::{self, DrawingArea, Window, WindowType};
use gtk::prelude::*;
use pango::{self, LayoutExt};
use pangocairo;

pub fn main() {
    gtk::init().unwrap();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("6term");
    let area = DrawingArea::new();
    window.add(&area);
    window.show_all();

    window.connect_delete_event(|_,_| {
        gtk::main_quit();
        Inhibit(false)
    });

    area.connect_draw(|_widget, cairo_ctx| {
        //draw background
        cairo_ctx.set_source_rgb(0., 0., 0.);
        cairo_ctx.paint();

        //draw foreground
        cairo_ctx.set_source_rgb(1., 1., 1.);
        cairo_ctx.move_to(100., 100.);

        let pango_ctx = pangocairo::functions::create_context(cairo_ctx).unwrap();
        let layout = pango::Layout::new(&pango_ctx);
        layout.set_text("Hello cruel world");

        let attr_list = pango::AttrList::new();
        let mut attr = pango::Attribute::new_strikethrough(true).unwrap();
        attr.set_start_index(6);
        attr.set_end_index(10);
        attr_list.insert(attr);
        layout.set_attributes(&attr_list);

        pangocairo::functions::show_layout(cairo_ctx, &layout);

        Inhibit(false)
    });

    gtk::main();
}
