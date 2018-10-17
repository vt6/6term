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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures::sync::mpsc;
use tokio::prelude::*;
use vt6;
use vt6::server::core::{StreamMode, StreamState, Tracker};
use vt6tokio;
use vt6tokio::server::core::IncomingEvent;

use model;
use window;

pub enum OutgoingEvent {
    RedrawWindow,
}

pub fn make_server_future(
    socket_path: PathBuf,
    incoming_rx: mpsc::Receiver<IncomingEvent>,
    model: Arc<Mutex<model::Document>>,
    window_handle: window::WindowHandle
) -> std::io::Result<impl Future<Item = (), Error = ()>> {
    let handler = vt6::server::RejectHandler {};
    let handler = vt6::server::core::Handler::new(handler);

    let (outgoing_tx, outgoing_rx) = mpsc::channel(10);

    //the first constituent future is the vt6tokio server
    let future1 = vt6tokio::server::core::Server::<Connection, _>::new(
        handler,
        socket_path,
        incoming_rx,
        outgoing_tx,
        model,
    )?;

    //the second constituent future consumes the outgoing_rx and emits
    //events onto the GTK eventloop
    let future2 = outgoing_rx.for_each(move |event| {
        use self::OutgoingEvent::*;
        match event {
            RedrawWindow => window_handle.redraw(),
        }
        Ok(())
    });

    //we run both futures to completion, but return () instead of ((), ())
    Ok(future1.join(future2).map(|_| ()))
}

////////////////////////////////////////////////////////////////////////////////
// Connection object

pub struct Connection {
    id: u32,
    tracker: Tracker,
    stream_state: StreamState,
    model: Arc<Mutex<model::Document>>,
    event_tx: mpsc::Sender<OutgoingEvent>,
}

impl vt6tokio::server::core::Connection for Connection {
    type ModelRef = Arc<Mutex<model::Document>>;
    type OutgoingEvent = OutgoingEvent;

    fn new(
        id: u32, model: Arc<Mutex<model::Document>>, event_tx: mpsc::Sender<OutgoingEvent>
    ) -> Connection
    {
        //first connection is the initial stdio
        let mode = if id == 0 { StreamMode::Stdio } else { StreamMode::Message };

        Connection {
            id, model, event_tx,
            tracker: Default::default(),
            stream_state: StreamState::enter(mode),
        }
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn handle_standard_output(&mut self, bytes_received: &[u8]) {
        let mut document = self.model.lock().unwrap();
        //append the received output to bottom-most output section
        if let Some(section) = document.sections.last_mut() {
            //TODO respect term.output-protected property
            section.append_output(bytes_received, false);
        }
        //TODO check return value from try_send
        self.event_tx.try_send(OutgoingEvent::RedrawWindow).unwrap();
    }
}

impl vt6::server::Connection for Connection {
    fn enable_module(&mut self, name: &str, version: vt6::common::core::ModuleVersion) {
        self.tracker.enable_module(name, version)
    }
    fn is_module_enabled(&self, name: &str) -> Option<vt6::common::core::ModuleVersion> {
        self.tracker.is_module_enabled(name)
    }

    fn stream_state(&self) -> StreamState {
        self.stream_state
    }
    fn set_stream_state(&mut self, value: StreamState) {
        self.stream_state = value;
    }
}

impl vt6::server::core::Connection for Connection {
    fn max_server_message_length(&self) -> usize { 1024 }
    fn max_client_message_length(&self) -> usize { 1024 }
}
