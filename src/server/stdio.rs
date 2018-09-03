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
use std::sync::{Arc, Mutex};

use tokio::prelude::*;
use tokio_uds::UnixStream;

use model;

///A connection that receives text from a client program's standard output and
///sends text to its standard input.
//TODO: rework into frame-scoped stdio later
//TODO: move to simple pipes instead of a Unix socket connection (when Tokio supports that)
pub struct Stdio {
    stream: UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl Stdio {
    pub fn new(stream: UnixStream) -> Stdio {
        Stdio {
            stream: stream,
            read_buffer: vec![0; 1024],
            write_buffer: Vec::new(),
        }
    }

    pub fn add_user_input(&mut self, t: &str) {
        self.write_buffer.extend(t.bytes());
    }

    pub fn poll(&mut self, model: &Arc<Mutex<model::Document>>, needs_redraw: &mut bool) -> Poll<(), std::io::Error> {
        let mut restart = false;

        //check if the client sent us some output
        match self.stream.poll_read(&mut self.read_buffer[..]) {
            Err(e) => return Err(e),
            Ok(Async::NotReady) => {},
            Ok(Async::Ready(bytes_read)) => {
                if bytes_read == 0 {
                    return Ok(Async::Ready(())); //EOF
                }
                let str_read = String::from_utf8_lossy(&self.read_buffer[0..bytes_read]);
                let mut document = model.lock().unwrap();
                //append the received output to bottom-most output section
                if let Some(section) = document.sections.last_mut() {
                    section.append_output(&str_read);
                }
                restart = true; //immediately try receiving more
                *needs_redraw = true; //instruct server to trigger redraw
            },
        }

        //check if we can send the client some input
        if self.write_buffer.len() > 0 {
            match self.stream.poll_write(&self.write_buffer[..]) {
                Err(e) => return Err(e),
                Ok(Async::NotReady) => {},
                Ok(Async::Ready(bytes_written)) => {
                    //remove the written bytes from the write buffer
                    self.write_buffer = self.write_buffer.split_off(bytes_written);
                    restart = true; //immediately try sending more
                },
            }
        }

        if restart {
            self.poll(model, needs_redraw)
        } else {
            Ok(Async::NotReady)
        }
    }
}