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

use std;

use tokio::prelude::*;
use tokio_io::AsyncRead;
use tokio_uds::UnixStream;
use vt6::core::msg;

pub struct Connection {
    id: u32,
    stream: UnixStream,
    recv: RecvBuffer,
}

impl Connection {
    pub fn new(id: u32, stream: UnixStream) -> Connection {
        trace!("connection {}: accepted", id);
        Connection {
            id: id,
            stream: stream,
            recv: RecvBuffer::new(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        info!("connection {} terminated", self.id);
    }
}

impl Future for Connection {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), std::io::Error> {
        //spell it out to the borrow checker that we're *not* borrowing `self` into the closure
        //below
        let self_id = self.id;

        self.recv.poll(&mut self.stream, self.id, |msg| {
            trace!("message received on connection {}: {}", self_id, msg);
            //TODO: pass `msg` to handler
        })
    }
}

////////////////////////////////////////////////////////////////////////////////

struct RecvBuffer {
    buf: Vec<u8>,
    fill: usize,
}

impl RecvBuffer {
    fn new() -> Self {
        RecvBuffer { buf: vec![0; 1024], fill: 0 }
    }

    ///Discards the given number of bytes from the buffer and shifts the remaining bytes to the
    ///left by that much.
    fn discard(&mut self, count: usize) {
        let remaining = self.fill - count;
        for idx in 0..remaining {
            self.buf[idx] = self.buf[idx + count];
        }
        for idx in remaining..self.buf.len() {
            self.buf[idx] = 0;
        }
        self.fill = remaining;
    }

    fn poll<R, F>(&mut self, reader: &mut R, connection_id: u32, mut handle_message: F) -> Poll<(), std::io::Error>
        where R: AsyncRead,
              F: FnMut(&msg::Message) {
        //NOTE: We cannot handle `bytes_to_discard` and `incomplete` directly
        //inside the match arms because the reference to `self.buf` needs to go
        //out of scope first.
        let (bytes_to_discard, incomplete) = match msg::Message::parse(&self.buf[0..self.fill]) {
            Ok((msg, bytes_consumed)) => {
                handle_message(&msg);
                (bytes_consumed, false)
            },
            Err(ref e) if e.kind == msg::ParseErrorKind::UnexpectedEOF && self.fill < self.buf.len() => {
                (0, true)
            },
            Err(e) => {
                //parser error -> reset the stream parser [vt6/core1.0; sect. 2.3]
                let bytes_to_discard = self.buf.iter().skip(1).position(|&c| c == b'{')
                    .map(|x| x + 1).unwrap_or(self.fill);
                //^ The .skip(1) is necessary to ensure that bytes_to_discard > 0.
                //The .map() compensates the effect of .skip(1) on the index.
                let discarded = String::from_utf8_lossy(&self.buf[0..bytes_to_discard]);
                error!("input discarded on connection {}: {:?}", connection_id, discarded);
                error!("-> reason: {}", e);
                (bytes_to_discard, false)
            },
        };

        if incomplete {
            //it appears we have not read a full message yet
            if self.fill < self.buf.len() {
                let bytes_read = try_ready!(reader.poll_read(&mut self.buf[self.fill..]));;
                self.fill += bytes_read;
                if bytes_read == 0 {
                    //EOF - if we still have something in the buffer, it's an
                    //unfinished message -> complain
                    if self.fill > 0 {
                        let err = msg::Message::parse(&self.buf[0..self.fill]).unwrap_err();
                        let discarded = String::from_utf8_lossy(&self.buf[0..self.fill]);
                        error!("input discarded on connection {}: {:?}", connection_id, discarded);
                        error!("-> reason: {}", err);
                    }
                    return Ok(Async::Ready(()));
                }
            }
            //restart handler with the new data
            return self.poll(reader, connection_id, handle_message);
        }

        //we have read something (either a message or a definitive parser
        //error), so now we need to discard the bytes that were processed from
        //the recv buffer
        self.discard(bytes_to_discard);
        //attempt to read the next message immediately
        self.poll(reader, connection_id, handle_message)
    }

}
