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
}

impl Stdio {
    pub fn new(stream: UnixStream) -> Stdio {
        Stdio {
            stream: stream,
            read_buffer: vec![0; 1024],
        }
    }

    pub fn poll(&mut self, model: &Arc<Mutex<model::Document>>) -> Poll<(), std::io::Error> {
        let mut restart = true;

        //check if the client send us some output
        match self.stream.poll_read(&mut self.read_buffer[..]) {
            Err(e) => return Err(e),
            Ok(Async::NotReady) => restart = false,
            Ok(Async::Ready(bytes_read)) => {
                if bytes_read == 0 {
                    return Ok(Async::Ready(())); //EOF
                }
                let str_read = String::from_utf8_lossy(&self.read_buffer[0..bytes_read]);
                let mut document = model.lock().unwrap();
                //append the received output to bottom-most output section
                for section in document.sections.iter_mut().rev() {
                    if section.disposition().contains(model::Disposition::NORMAL_OUTPUT) {
                        section.append_text(str_read);
                        break;
                    }
                }
                //TODO: schedule a re-render in the GUI thread
            },
        }

        //TODO: check if we can send the client some input

        if restart {
            self.poll(model)
        } else {
            Ok(Async::NotReady)
        }
    }
}
