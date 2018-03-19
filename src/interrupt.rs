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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use simple_signal::{self, Signal};
use tokio::prelude::*;

#[derive(Clone,Debug)]
pub(crate) struct Interrupt {
    triggered: Arc<AtomicBool>,
}

impl Interrupt {
    pub fn new() -> Interrupt {
        let result = Interrupt {
            triggered: Arc::new(AtomicBool::new(false))
        };
        let t = result.triggered.clone();
        simple_signal::set_handler(
            &[Signal::Int, Signal::Term],
            move |_| { t.store(true, Ordering::SeqCst); }
        );
        result
    }
}

unsafe impl Send for Interrupt {
}

///This is a task. It completes when SIGINT or SIGTERM has been pressed.
impl future::Future for Interrupt {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<()>, ()> {
        if self.triggered.load(Ordering::SeqCst) {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}
