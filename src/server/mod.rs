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

mod connection;

use std;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Mutex;

use futures::sync::oneshot;
use simple_signal::{self, Signal};
use tokio;
use tokio::prelude::*;
use tokio_core::reactor::Handle;
use tokio_uds::UnixListener;

pub struct Config<'a> {
    pub socket_path: &'a Path,
}

pub fn run<'a>(handle: &'a Handle, cfg: Config<'a>) -> std::io::Result<Box<Future<Item=(), Error=()> + 'a>> {
    //FIXME This opens the socket with SOCK_STREAM, but vt6/posix1 mandates SOCK_SEQPACKET.
    //I'm doing the prototyping with this for now because neither mio-uds nor tokio-uds support
    //SOCK_SEQPACKET.
    let listener = UnixListener::bind(cfg.socket_path, handle)?;

    //setup a signal handler to cleanly shutdown the server when SIGINT or
    //SIGTERM is received
    let (interrupt_tx, interrupt_rx) = oneshot::channel();
    let interrupt_tx: RefCell<Option<oneshot::Sender<()>>> = RefCell::new(Some(interrupt_tx));
    simple_signal::set_handler(
        &[Signal::Int, Signal::Term],
        move |_| {
            if let Some(tx) = interrupt_tx.replace(None) {
                tx.send(()).expect("Interrupt report failed");
            }
        }
    );

    let next_connection_id = Mutex::new(1);

    let server = listener.incoming()
        .map_err(|err| { error!("listener.incoming.for_each: {}", err); })
        .for_each(move |(stream, _addr)| {
            let mut next_connection_id = next_connection_id.lock().unwrap();
            let connection_id = *next_connection_id;
            *next_connection_id += 1;

            tokio::spawn(connection::Connection::new(connection_id, stream)
                .map_err(move |err| { error!("fatal error on connection {}: {}", connection_id, err); })
            );
            Ok(())
        });

    //stop the eventloop either when the `server` future returns, or when an
    //interrupt is received
    let interrupt_future = interrupt_rx
        .map_err(|err| { error!("interrupt channel canceled: {}", err); });
    let server = server.select(interrupt_future).map_err(|_| ());

    //cleanup phase
    let server = server.and_then(move |_| {
        if let Err(err) = std::fs::remove_file(cfg.socket_path) {
            error!("socket cleanup failed: {}", err);
        }
        Ok(())
    });

    return Ok(Box::new(server));
}
