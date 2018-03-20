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

extern crate futures;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate simple_signal;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_uds;

mod server;

use tokio_core::reactor::Core;

fn main() {
    simple_logger::init().unwrap();

    if let Err(err) = run() {
        error!("{}", err);
    }
}

fn run() -> std::io::Result<()> {
    let mut core = Core::new().expect("Core::new failed");
    let handle = core.handle();

    let socket_path = std::path::Path::new("./vt6term");
    let server = server::run(&handle, server::Config {
        socket_path: &socket_path,
    })?;

    core.run(server).expect("Event loop failed");
    Ok(())
}
