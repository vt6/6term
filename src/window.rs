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

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use futures::sync::mpsc;
use gdk;
use gtk::{self, DrawingArea, Window, WindowType};
use gtk::prelude::*;

use model::{CursorAction, Document};
use server;
use view;

///Returns when the GUI thread is done, meaning that all other threads shall be shut down.
pub fn main(_tx: &mut mpsc::Sender<server::Event>, document_ref: Arc<Mutex<Document>>) {
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

    let view = Rc::new(RefCell::new(
        view::Document::new(document_ref.clone()),
    ));

    area.connect_draw(move |widget, cairo_ctx| {
        view.borrow_mut().render(widget, cairo_ctx);
        Inhibit(false)
    });

    area.add_events(gdk::EventMask::KEY_PRESS_MASK.bits() as i32);
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
        let mut document = document_ref.lock().unwrap();
        if let Some(ref mut p) = document.sections.last_mut() {
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
