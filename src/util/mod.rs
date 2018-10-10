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

use std::convert::AsRef;
use std::ops::Deref;
use std::sync::Arc;
use std::thread::{self, ThreadId};

//An Arc<T> for types T that are not Send. It can be cloned everywhere, but T can
//only be accessed in the thread where the AnchoredArc was originally created.
#[derive(Clone)]
pub struct AnchoredArc<T>(Arc<T>, ThreadId);

impl<T> AnchoredArc<T> {
    pub fn new(value: T) -> AnchoredArc<T> {
        AnchoredArc(Arc::new(value), thread::current().id())
    }
}

unsafe impl<T> Send for AnchoredArc<T> {}
unsafe impl<T> Sync for AnchoredArc<T> {}

impl<T> AsRef<T> for AnchoredArc<T> {
    fn as_ref(&self) -> &T {
        let cur = thread::current().id();
        if self.1 != cur {
            panic!("trying to unpack AnchoredArc from thread {:?} on thread {:?}", self.1, cur);
        }
        self.0.as_ref()
    }
}

impl<T> Deref for AnchoredArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}
