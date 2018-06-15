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
extern crate futures;
extern crate glib;
extern crate gtk;
#[macro_use]
extern crate log;
extern crate pango;
extern crate pangocairo;
extern crate simple_logger;
extern crate simple_signal;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_uds;
extern crate vt6;

mod server;
mod window;

fn main() {
    simple_logger::init().unwrap();

    //TODO: shutdown this thread when the GUI thread is done
    std::thread::spawn(|| {
        if let Err(err) = run() {
            error!("{}", err);
        }
    });

    window::main();
}

fn run() -> std::io::Result<()> {
    let socket_path = std::path::Path::new("./vt6term");
    let server = server::run(server::Config {
        socket_path: &socket_path,
    })?;

    use futures::Future;
    use tokio::runtime::Runtime;

    let mut rt = Runtime::new().unwrap();
    rt.block_on(server).unwrap();
    rt.shutdown_now().wait().unwrap();
    Ok(())
}
