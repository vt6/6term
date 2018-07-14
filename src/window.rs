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
use glib;
use gtk::{self, DrawingArea, Window as GtkWindow, WindowType};
use gtk::prelude::*;

use model;
use server;
use util::AnchoredArc;
use view;

pub struct Window {
    window: GtkWindow,
    area: AnchoredArc<DrawingArea>,
}

impl Window {
    pub fn new() -> Window {
        gtk::init().unwrap();

        let w = Window {
            window: GtkWindow::new(WindowType::Toplevel),
            area: AnchoredArc::new(DrawingArea::new()),
        };
        w.window.set_title("6term");
        w.window.add(w.area.as_ref());
        w.window.show_all();

        w
    }

    pub fn handle(&self) -> WindowHandle {
        WindowHandle(self.area.clone())
    }

    ///Returns when the GUI thread is done, meaning that all other threads shall be shut down.
    pub fn main(&mut self, tx: mpsc::Sender<server::Event>, model: Arc<Mutex<model::Document>>) {

        self.window.connect_delete_event(|_,_| {
            gtk::main_quit();
            Inhibit(false)
        });

        let view = Rc::new(RefCell::new(
            view::Document::new(model.clone()),
        ));

        self.area.connect_draw(move |widget, cairo_ctx| {
            view.borrow_mut().render(widget, cairo_ctx);
            Inhibit(false)
        });

        let tx = Rc::new(RefCell::new(tx));

        self.area.add_events(gdk::EventMask::KEY_PRESS_MASK.bits() as i32);
        self.area.connect_key_press_event(move |widget, event| {
            let keyval = event.get_keyval();
            let action = match gdk::keyval_to_unicode(keyval) {
                //Enter or Return
                Some('\n') | Some('\r') => model::CursorAction::Insert("\n".into()),
                //Backspace
                Some('\u{8}') => model::CursorAction::DeletePreviousChar,
                //Delete
                Some('\u{7F}') => model::CursorAction::DeleteNextChar,
                //printable character
                Some(ch) if ch as u32 >= 32 => model::CursorAction::Insert(ch.to_string()),
                //ignore other control characters
                Some(_) => return Inhibit(false),
                //other keys
                None => {
                    use gdk::enums::key;
                    match keyval as key::Key {
                        key::Left  | key::KP_Left  => model::CursorAction::GotoPreviousChar,
                        key::Right | key::KP_Right => model::CursorAction::GotoNextChar,
                        _ => {
                            info!("unhandled keyval: {}", keyval);
                            return Inhibit(false);
                        },
                    }
                },
            };
            let mut document = model.lock().unwrap();
            if let Some(ref mut p) = document.sections.last_mut() {
                match p.execute_cursor_action(action) {
                    model::CursorActionResult::Unchanged => {},
                    model::CursorActionResult::Changed => widget.queue_draw(),
                    model::CursorActionResult::LineCompleted(s) => {
                        widget.queue_draw();
                        //TODO check return value from try_send
                        tx.borrow_mut().try_send(server::Event::UserInput(s)).unwrap();
                    },
                }
            }
            Inhibit(true)
        });

        self.area.set_can_focus(true);
        self.area.grab_focus();

        gtk::main();
    }
}

pub struct WindowHandle(AnchoredArc<DrawingArea>);

impl WindowHandle {
    ///Can be called by any thread to trigger a redraw of the GUI.
    pub fn redraw(&self) {
        let anchored_arc = self.0.clone();
        glib::idle_add(move || {
            anchored_arc.queue_draw();
            Continue(false)
        });
    }
}
