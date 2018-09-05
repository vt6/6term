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
use tokio::io::{ReadHalf, WriteHalf};
use tokio_uds::UnixStream;
use vt6;
use vt6::core::msg;

use server::connection_state::*;

pub struct Connection {
    state: ConnectionState,
    recv: RecvBuffer<UnixStream>,
    send: SendBuffer<UnixStream>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        info!("connection {} terminated", self.state.id());
    }
}

impl Connection {
    pub fn new(id: u32, stream: UnixStream) -> Connection {
        trace!("connection {}: accepted", id);
        let (reader, writer) = stream.split();
        Connection {
            state: ConnectionState::new(id),
            recv: RecvBuffer::new(reader),
            send: SendBuffer::new(writer),
        }
    }

    pub fn id(&self) -> u32 {
        self.state.id()
    }

    pub fn poll(&mut self, handler: &vt6::server::EarlyHandler<ConnectionState>) -> Poll<(), std::io::Error> {
        //spell it out to the borrow checker that we're *not* borrowing `self`
        //into the closure below
        let self_id = self.state.id();
        let self_send = &mut self.send;
        let self_state = &mut self.state;

        let recv_result = self.recv.poll(self_id, |msg| {
            trace!("message received on connection {}: {}", self_id, msg);

            //if the send buffer is getting full, try to empty it before
            //handling the message (we want to guarantee at least 1024 bytes in
            //the send buffer before trying to handle the message)
            while self_send.0.unfilled_len() < 1024 {
                try_ready!(self_send.poll());
            }

            //try to handle this messag
            let result = handler.handle(msg, self_state, self_send.0.unfilled_mut());
            match result {
                Some(bytes_written) => {
                    self_send.0.fill += bytes_written;
                    //TODO validate that self_send.fill < self_send.buf.len()
                },
                None => {
                    //message was either invalid or the send buffer was exceeded
                    //when trying to send a reply -> answer with (nope) instead
                    let result = msg::MessageFormatter::new(self_send.0.unfilled_mut(),"nope", 0).finalize();
                    if let Ok(bytes_written) = result { // TODO otherwise log error
                        self_send.0.fill += bytes_written;
                    }
                },
            };
            Ok(Async::Ready(()))
        });

        if let Ok(Async::NotReady) = recv_result {
            //when self.recv.poll() returned "not ready", make sure that the
            //task also knows about our interest in writing to self.writer
            if self_send.0.fill > 0 {
                //note that this never returns Async::Ready
                return self_send.poll();
            }
        }
        recv_result
    }
}

////////////////////////////////////////////////////////////////////////////////

struct Buffer {
    buf: Vec<u8>,
    fill: usize,
}

impl Buffer {
    fn new(size: usize) -> Self {
        Self { buf: vec![0; size], fill: 0 }
    }

    //assorted helper methods
    fn unfilled_len(&self) -> usize { self.buf.len() - self.fill }
    fn leading(&self, bytes: usize) -> &[u8] { &self.buf[0 .. bytes] }
    fn filled(&self) -> &[u8] { self.leading(self.fill) }
    fn unfilled_mut(&mut self) -> &mut [u8] { &mut self.buf[self.fill ..] }

    ///Discards the given number of bytes from the buffer and shifts the
    ///remaining bytes to the left by that much.
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

////////////////////////////////////////////////////////////////////////////////

struct RecvBuffer<T: AsyncRead>(Buffer, ReadHalf<T>);

impl<T: AsyncRead> RecvBuffer<T> {
    fn new(writer: ReadHalf<T>) -> Self {
        RecvBuffer(Buffer::new(2048), writer)
    }

    fn poll<F>(&mut self, connection_id: u32, mut handle_message: F) -> Poll<(), std::io::Error>
        where F: FnMut(&msg::Message) -> Poll<(), std::io::Error> {
        //NOTE: We cannot handle `bytes_to_discard` and `incomplete` directly
        //inside the match arms because the reference to `self.0.buf` needs to go
        //out of scope first.
        let (bytes_to_discard, incomplete) = match msg::Message::parse(self.0.filled()) {
            Ok((msg, bytes_consumed)) => {
                try_ready!(handle_message(&msg));
                (bytes_consumed, false)
            },
            Err(ref e) if e.kind == msg::ParseErrorKind::UnexpectedEOF && self.0.unfilled_len() > 0 => {
                (0, true)
            },
            Err(e) => {
                //parser error -> reset the stream parser [vt6/core1.0; sect. 2.3]
                let bytes_to_discard = self.0.buf.iter().skip(1).position(|&c| c == b'{')
                    .map(|x| x + 1).unwrap_or(self.0.fill);
                //^ The .skip(1) is necessary to ensure that bytes_to_discard > 0.
                //The .map() compensates the effect of .skip(1) on the index.
                let discarded = String::from_utf8_lossy(self.0.leading(bytes_to_discard));
                error!("input discarded on connection {}: {:?}", connection_id, discarded);
                error!("-> reason: {}", e);
                (bytes_to_discard, false)
            },
        };

        if incomplete {
            //it appears we have not read a full message yet
            if self.0.unfilled_len() > 0 {
                let bytes_read = try_ready!(self.1.poll_read(self.0.unfilled_mut()));;
                self.0.fill += bytes_read;
                if bytes_read == 0 {
                    //EOF - if we still have something in the buffer, it's an
                    //unfinished message -> complain
                    if self.0.fill > 0 {
                        let err = msg::Message::parse(self.0.filled()).unwrap_err();
                        let discarded = String::from_utf8_lossy(self.0.filled());
                        error!("input discarded on connection {}: {:?}", connection_id, discarded);
                        error!("-> reason: {}", err);
                    }
                    return Ok(Async::Ready(()));
                }
            }
            //restart handler with the new data
            return self.poll(connection_id, handle_message);
        }

        //we have read something (either a message or a definitive parser
        //error), so now we need to discard the bytes that were processed from
        //the recv buffer
        self.0.discard(bytes_to_discard);
        //attempt to read the next message immediately
        self.poll(connection_id, handle_message)
    }

}

////////////////////////////////////////////////////////////////////////////////

struct SendBuffer<T: AsyncWrite>(Buffer, WriteHalf<T>);

impl<T: AsyncWrite> SendBuffer<T> {
    fn new(writer: WriteHalf<T>) -> Self {
        SendBuffer(Buffer::new(2048), writer)
    }

    fn poll(&mut self) -> Poll<(), std::io::Error> {
        let bytes_sent = try_ready!(self.1.poll_write(self.0.filled()));
        self.0.discard(bytes_sent);
        Ok(Async::NotReady) //we can always add more stuff to the send buffer
    }
}
