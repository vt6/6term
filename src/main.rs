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

extern crate cairo;
#[macro_use]
extern crate futures;
extern crate gdk;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate log;
extern crate pango;
extern crate pangocairo;
extern crate simple_logger;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_uds;
extern crate vt6;

mod document;
mod server;
mod window;

use futures::sync::mpsc;

fn main() {
    simple_logger::init().unwrap();
    //setup channel for communication from GUI thread to Tokio eventloop
    let (mut event_tx, event_rx) = mpsc::channel(10);

    let socket_path = std::path::PathBuf::from("./vt6term");
    let server = match server::Server::new(socket_path, event_rx) {
        Ok(s) => s,
        Err(e) => {
            error!("failed to initialize VT6 server socket: {}", e);
            std::process::exit(1);
        },
    };

    //TODO: shutdown this thread when the GUI thread is done
    let join_handle = std::thread::spawn(move || {
        use futures::Future;
        use tokio::runtime::Runtime;

        let mut rt = Runtime::new().unwrap();
        rt.block_on(server).unwrap();
        rt.shutdown_now().wait().unwrap();
    });

    window::main(&mut event_tx);
    std::mem::drop(event_tx); //signal to server future to shutdown
    join_handle.join().unwrap();
}
