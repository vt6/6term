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
use pangocairo;
use std::cell::RefCell;
use std::rc::Rc;

use document::paragraph::Paragraph;

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

    let paragraphs = Rc::new(RefCell::new(vec![
        Paragraph::new("Lorem ipsum dolor sit amet,".into()),
        Paragraph::new("consectetuer adipiscing elit.".into()),
        Paragraph::new("Lorem ipsum dolor sit amet, consectetuer adipiscing elit.".into()),
    ]));

    area.connect_draw(move |widget, cairo_ctx| {
        let pixel_width = widget.get_allocated_width();

        //draw background
        cairo_ctx.set_source_rgb(0., 0., 0.);
        cairo_ctx.paint();

        //draw paragraphs
        cairo_ctx.set_source_rgb(1., 1., 1.);
        cairo_ctx.move_to(0., 0.);

        let pango_ctx = pangocairo::functions::create_context(cairo_ctx).unwrap();
        for paragraph in paragraphs.borrow_mut().iter_mut() {
            let height = paragraph.prepare_rendering(pixel_width, &pango_ctx);
            paragraph.render(cairo_ctx);
            cairo_ctx.rel_move_to(0., height as f64);
        }

        /* TODO kept for later reference
        let attr_list = pango::AttrList::new();
        let mut attr = pango::Attribute::new_strikethrough(true).unwrap();
        attr.set_start_index(6);
        attr.set_end_index(10);
        attr_list.insert(attr);
        layout.set_attributes(&attr_list);
        */

        Inhibit(false)
    });

    gtk::main();
}
