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
use std::path::PathBuf;

use futures::sync::mpsc;
use tokio::prelude::*;
use tokio_uds::UnixListener;

use self::connection::Connection;

pub enum Event {}

pub struct Server {
    socket_path: PathBuf,
    socket: UnixListener,
    connections: Vec<Connection>,
    next_connection_id: u32,
    event_rx: mpsc::Receiver<Event>,
}

impl Server {
    pub fn new(socket_path: PathBuf, rx: mpsc::Receiver<Event>) -> std::io::Result<Self> {
        //FIXME This opens the socket with SOCK_STREAM, but vt6/posix1 mandates
        //SOCK_SEQPACKET. I'm doing the prototyping with this for now because
        //neither mio-uds nor tokio-uds support SOCK_SEQPACKET.
        let listener = UnixListener::bind(&socket_path)?;

        Ok(Server {
            socket_path: socket_path,
            socket: listener,
            connections: Vec::new(),
            next_connection_id: 0,
            event_rx: rx,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.socket_path) {
            error!("socket cleanup failed: {}", err);
        }
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        //check for new client connections
        match self.socket.poll_accept() {
            Err(e) => {
                error!("error accepting new client connection: {}", e);
                return Err(()); //this error is fatal (TODO: report on GUI)
            },
            Ok(Async::Ready((stream, _))) => {
                self.next_connection_id += 1;
                self.connections.push(Connection::new(self.next_connection_id, stream));
            },
            _ => {},
        };

        //recurse into client connections to handle input received on them
        let mut closed_connection_ids = std::collections::hash_set::HashSet::new();
        for c in self.connections.iter_mut() {
            match c.poll() {
                Err(e) => {
                    error!("error on connection {}: {}", c.id(), e);
                    //fatal error for this connection - close it from our side
                    closed_connection_ids.insert(c.id());
                },
                Ok(Async::Ready(())) => {
                    //client disconnected
                    closed_connection_ids.insert(c.id());
                },
                Ok(Async::NotReady) => {},
            }
        }
        self.connections.retain(|ref c| !closed_connection_ids.contains(&c.id()) );

        //see if there's any events we need to react to
        match self.event_rx.poll() {
            Err(e) => {
                error!("error receiving events from frontend: {:?}", e);
                Err(()) //this error is fatal (TODO: report on GUI)
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            //closed channel signals shutdown request from GUI thread
            Ok(Async::Ready(None)) => Ok(Async::Ready(())),
            Ok(Async::Ready(Some(_))) => Ok(Async::NotReady), //TODO placeholder
        }
    }
}
