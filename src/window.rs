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

use document::section::{CursorAction, Section};
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

    let sections = Rc::new(RefCell::new(vec![
        Section::new("Lorem ipsum dolor sit amet,".into()),
        Section::new("consectetuer adipiscing elit.".into()),
        Section::new("Lorem ipsum dolor sit amet, consectetuer adipiscing elit.".into()),
    ]));

    let sections1 = sections.clone();
    area.connect_draw(move |widget, cairo_ctx| {
        let pixel_width = widget.get_allocated_width();

        //draw background
        cairo_ctx.set_source_rgb(0., 0., 0.);
        cairo_ctx.paint();

        //draw sections
        cairo_ctx.set_source_rgb(1., 1., 1.);
        cairo_ctx.move_to(0., 0.);

        let pango_ctx = pangocairo::functions::create_context(cairo_ctx).unwrap();
        for section in sections1.borrow_mut().iter_mut() {
            let height = section.prepare_rendering(pixel_width, &pango_ctx);
            section.render(cairo_ctx);
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
    let sections2 = sections.clone();
    area.connect_key_press_event(move |widget, event| {
        let keyval = event.get_keyval();
        let action = match gdk::keyval_to_unicode(keyval) {
            //Enter or Return
            Some('\n') | Some('\r') => CursorAction::Insert("\n".into()),
            //Backspace
            Some('\u{8}') => CursorAction::DeletePreviousChar,
            //Delete
            Some('\u{7F}') => CursorAction::DeleteNextChar,
            //printable character
            Some(ch) if ch as u32 >= 32 => CursorAction::Insert(ch.to_string()),
            //ignore other control characters
            Some(_) => return Inhibit(false),
            //other keys
            None => {
                use gdk::enums::key;
                match keyval as key::Key {
                    key::Left  | key::KP_Left  => CursorAction::GotoPreviousChar,
                    key::Right | key::KP_Right => CursorAction::GotoNextChar,
                    _ => {
                        info!("unhandled keyval: {}", keyval);
                        return Inhibit(false);
                    },
                }
            },
        };
        if let Some(ref mut p) = sections2.borrow_mut().last_mut() {
            let need_redraw = p.execute_cursor_action(action);
            if need_redraw {
                widget.queue_draw();
            }
        }
        Inhibit(true)
    });

    area.set_can_focus(true);
    area.grab_focus();

    gtk::main();
}
