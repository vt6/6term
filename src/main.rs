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

#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate simple_signal;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_uds;

mod interrupt;

use tokio::prelude::*;
use tokio_core::reactor::Core;
use tokio_uds::UnixListener;

fn main() {
    simple_logger::init().unwrap();

    if let Err(err) = run() {
        error!("{}", err);
    }
}

struct CleanupSocketOnExit<'a>(&'a std::path::Path);

impl<'a> Drop for CleanupSocketOnExit<'a> {
    fn drop(&mut self) {
        std::fs::remove_file(self.0).expect("Socket cleanup failed");
    }
}

fn run() -> std::io::Result<()> {
    let mut core = Core::new().expect("Core::new failed");
    let handle = core.handle();

    //TODO: the Drop handler does not run when SIGINT/SIGTERM terminates the program
    let socket_path = CleanupSocketOnExit(std::path::Path::new("./vt6term"));
    let listener = UnixListener::bind(socket_path.0, &handle)?;

    let server = listener.incoming().for_each(|(stream, addr)| {
        trace!("accepted client connection: {:?}", addr);
        let (reader, writer) = stream.split();
        tokio::spawn(
            tokio::io::copy(reader, writer)
                .map(|_| ())
                .map_err(|err| { error!("stream copy: {}", err); () })
        );
        Ok(())
    }).map_err(|err| {
        error!("listener.incoming.for_each: {}", err);
    });

    //stop the eventloop either when the `server` future returns, or when an
    //interrupt is received
    let interrupt_future = interrupt::Interrupt::new();
    let server = interrupt_future.select(server).map_err(|_| ());

    core.run(server).expect("Event loop failed");
    Ok(())
}
