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
extern crate bitflags;
extern crate cairo;
extern crate fragile;
#[macro_use]
extern crate futures;
extern crate gdk;
extern crate glib;
extern crate gtk;
extern crate libc;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pango;
extern crate pangocairo;
extern crate simple_logger;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_uds;
extern crate vt6;

mod model;
mod server;
mod view;
mod window;

use futures::sync::mpsc;

fn main() {
    simple_logger::init().unwrap();

    //setup the model shared by all threads
    let model = model::Document::new();
    {
        let mut document = model.lock().unwrap();
        let s = document.make_section("Lorem ipsum dolor sit amet,".into(), model::Disposition::NORMAL_OUTPUT);
        document.sections.push(s);
        let s = document.make_section("consectetuer adipiscing elit.\n".into(), model::Disposition::NORMAL_OUTPUT);
        document.sections.push(s);
        let s = document.make_section("Lorem ipsum dolor sit amet, consectetuer adipiscing elit.".into(), model::Disposition::CANONICAL_INPUT);
        document.sections.push(s);
    } //drop MutexGuard<Document>

    //setup channel for communication from GUI thread to Tokio eventloop
    let (mut event_tx, event_rx) = mpsc::channel(10);


    let socket_path = std::path::PathBuf::from("./vt6term");
    let server = match server::Server::new(socket_path.clone(), event_rx, model.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("failed to initialize VT6 server socket: {}", e);
            std::process::exit(1);
        },
    };

    let mut win = window::Window::new();

    let join_handle1 = std::thread::spawn(move || {
        use futures::Future;
        use tokio::runtime::Runtime;

        let mut rt = Runtime::new().unwrap();
        rt.block_on(server).unwrap();
        rt.shutdown_now().wait().unwrap();
    });

    let join_handle2 = std::thread::spawn(move || {
        let result = spawn_client(socket_path, vec!["/bin/sh".into(), "-i".into()]);
        use nix::sys::wait::WaitStatus::*;
        match result {
            Err(e) => error!("spawn_client failed: {}", e),
            Ok(Exited(_, code)) => info!("client exited with status {}", code),
            Ok(Signaled(_, code, _)) => info!("client killed by signal {:?}", code),
            _ => info!("client watcher returned with status {:?}", result),
        }
    });

    win.main(&mut event_tx, model);
    std::mem::drop(event_tx); //signal to server future to shutdown
    join_handle1.join().unwrap();
    join_handle2.join().unwrap();
}

use std::ffi::CString;

fn spawn_client(socket_path: std::path::PathBuf, command_and_args: Vec<String>) -> nix::Result<nix::sys::wait::WaitStatus> {
    //before forking, make all necessary allocations
    let env: Vec<CString> = std::env::vars()
        .filter(|(k,_v)| k != "SHELL" && k != "LINES" && k != "COLUMNS")
        .map(|(k,v)| CString::new(format!("{}={}", k, v)).unwrap())
        .collect();
    let args: Vec<CString> = command_and_args.iter().map(|s| CString::new(s.clone()).unwrap()).collect();
    let command = CString::new(command_and_args[0].clone()).unwrap();

    //open stdio for the child process
    use std::os::unix::io::IntoRawFd;
    use std::os::unix::net::UnixStream;
    let stream = UnixStream::connect(socket_path).unwrap();

    use nix::unistd::*;
    match fork()? {
        ForkResult::Parent { child, .. } => {
            //wait on child
            nix::sys::wait::waitpid(child, None)
        },
        ForkResult::Child => {
            let stream_fd = stream.into_raw_fd();
            dup2(stream_fd, 0).unwrap_or_else(|_| std::process::exit(200));
            dup2(stream_fd, 1).unwrap_or_else(|_| std::process::exit(201));
            dup2(stream_fd, 2).unwrap_or_else(|_| std::process::exit(202));
            if stream_fd > 2 {
                close(stream_fd).unwrap_or_else(|_| std::process::exit(203));
            }
            execve(&command, &args, &env).unwrap_or_else(|_| std::process::exit(204));
            //FIXME remove the next line when rustc learns to understand that Void can cast into anything
            Ok(nix::sys::wait::WaitStatus::StillAlive)
        },
    }
}
