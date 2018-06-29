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

use futures::sync::mpsc;
use gdk;
use gtk::{self, DrawingArea, Window, WindowType};
use gtk::prelude::*;
use pangocairo;
use std::cell::RefCell;
use std::rc::Rc;

use document::paragraph::Paragraph;
use server;

///Returns when the GUI thread is done, meaning that all other threads shall be shut down.
pub fn main(_tx: &mut mpsc::Sender<server::Event>) {
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

    let paragraphs1 = paragraphs.clone();
    area.connect_draw(move |widget, cairo_ctx| {
        let pixel_width = widget.get_allocated_width();

        //draw background
        cairo_ctx.set_source_rgb(0., 0., 0.);
        cairo_ctx.paint();

        //draw paragraphs
        cairo_ctx.set_source_rgb(1., 1., 1.);
        cairo_ctx.move_to(0., 0.);

        let pango_ctx = pangocairo::functions::create_context(cairo_ctx).unwrap();
        for paragraph in paragraphs1.borrow_mut().iter_mut() {
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

    area.add_events(gdk::EventMask::KEY_PRESS_MASK.bits() as i32);
    let paragraphs2 = paragraphs.clone();
    area.connect_key_press_event(move |widget, event| {
        let need_redraw = match gdk::keyval_to_unicode(event.get_keyval()) {
            Some('\n') | Some('\r') => { // Enter/Return
                if let Some(ref mut p) = paragraphs2.borrow_mut().last_mut() {
                    p.text_mut().push('\n');
                }
                true
            },
            Some('\u{8}') => { // backspace
                //TODO this is probably wrong for grapheme clusters
                if let Some(ref mut p) = paragraphs2.borrow_mut().last_mut() {
                    p.text_mut().pop().is_some()
                } else {
                    false
                }
            },
            Some(ch) if ch as u32 >= 32 => { // printable character
                if let Some(ref mut p) = paragraphs2.borrow_mut().last_mut() {
                    p.text_mut().push(ch);
                }
                true
            },
            _ => false,
        };
        if need_redraw {
            widget.queue_draw();
        }
        Inhibit(true)
    });

    area.set_can_focus(true);
    area.grab_focus();

    gtk::main();
}
