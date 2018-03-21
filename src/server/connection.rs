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
use std::vec::Vec;

use tokio::prelude::*;
use tokio_io::{io, AsyncRead};
use tokio_uds::UnixStream;
use vt6::core::msg;

pub struct Connection {
    id: u32,
    stream: UnixStream,
    recv_buffer: RecvBuffer,
}

impl Connection {
    pub fn new(id: u32, stream: UnixStream) -> Connection {
        trace!("connection {}: accepted", id);
        Connection {
            id: id,
            stream: stream,
            recv_buffer: RecvBuffer::new(),
        }
    }
}

impl Future for Connection {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), std::io::Error> {
        match try_ready!(self.recv_buffer.poll_recv(&mut self.stream)) {
            RecvItem::SExpression(sexp) => {
                info!("s-expr received on connection {}: {}", self.id, sexp);
                //TODO: do something with it
            },
            RecvItem::Discarded(text, err) => {
                error!("input discarded on connection {}: {:?}", self.id, text);
                error!("-> reason: {}", err);
            },
        }

        //attempt to read next message immediately
        self.poll()
    }
}

enum RecvItem {
    SExpression(msg::SExpression),
    Discarded(String, msg::ParseError),
}

struct RecvBuffer {
    buf: Vec<u8>,
    fill: usize,
}

impl RecvBuffer {
    fn new() -> Self {
        RecvBuffer { buf: vec![0; 1024], fill: 0 }
    }

    pub fn poll_recv<R>(&mut self, reader: &mut R) -> Poll<RecvItem, std::io::Error>
        where R: AsyncRead
    {
        let (parse_result, bytes_consumed) = {
            let mut state = msg::ParserState::new(&self.buf[0..self.fill]);
            let result = msg::SExpression::parse(&mut state);
            (result, state.cursor)
        };

        match parse_result {
            Ok(sexp) => {
                self.discard(bytes_consumed);
                Ok(Async::Ready(RecvItem::SExpression(sexp)))
            },
            Err(ref e) if e.kind == msg::ParseErrorKind::UnexpectedEOF && self.fill < self.buf.len() => {
                //we have not read the entire message yet
                if self.fill < self.buf.len() {
                    self.fill += try_ready!(reader.poll_read(&mut self.buf[self.fill..]));;
                }
                self.poll_recv(reader)
            },
            Err(e) => {
                //parser error -> reset the stream parser [vt6/core1.0; sect. 2.4]
                let bytes_to_discard = self.buf.iter().position(|&c| c == b'(').unwrap_or(self.fill);
                let discarded = String::from_utf8_lossy(&self.buf[0..bytes_to_discard]).into();
                self.discard(bytes_to_discard);
                Ok(Async::Ready(RecvItem::Discarded(discarded, e)))
            },
        }
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
}
