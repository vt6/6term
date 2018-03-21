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

use std::vec::Vec;

use tokio;
use tokio::prelude::*;
use tokio_uds::UnixStream;

pub struct Connection {
    id: u32,
    stream: UnixStream,
    recv_buffer: Vec<u8>,
}

impl Connection {
    pub fn new(id: u32, stream: UnixStream) -> Connection {
        trace!("connection {}: accepted", id);
        Connection {
            id: id,
            stream: stream,
            recv_buffer: Vec::with_capacity(1024),
        }
    }

    pub fn exec(self) {
        //TODO replace placeholder echo server by actual VT6 server behavior
        let (reader, writer) = self.stream.split();
        let id = self.id;
        tokio::spawn(
            tokio::io::copy(reader, writer)
                .map(|_| ())
                .map_err(move |err| { error!("stream copy on connection {}: {}", id, err); () })
        );
    }
}
