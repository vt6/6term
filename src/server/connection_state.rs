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

use vt6;
use server::term_handler as th;

pub struct ConnectionState {
    id: u32,
    tracker: vt6::server::core::Tracker,
    is_stdio: bool,
    //TODO conn-lt-scoped properties from term
}

impl ConnectionState {
    pub fn new(id: u32) -> Self {
        ConnectionState {
            id: id,
            is_stdio: false,
            tracker: Default::default(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl vt6::server::Connection for ConnectionState {
    fn max_server_message_length(&self) -> usize { 1024 }
    fn max_client_message_length(&self) -> usize { 1024 }

    fn enable_module(&mut self, name: &str, version: vt6::common::core::ModuleVersion) {
        self.tracker.enable_module(name, version)
    }
    fn is_module_enabled(&self, name: &str) -> Option<vt6::common::core::ModuleVersion> {
        self.tracker.is_module_enabled(name)
    }
}

impl th::TermConnection for ConnectionState {
    fn is_input_echo(&self) -> bool { false /* TODO */ }
    fn is_input_immediate(&self) -> bool { false /* TODO */ }
    fn is_output_protected(&self) -> bool { false /* TODO */ }

    fn is_output_reflow(&self) -> bool { true }
    fn is_output_wordwrap(&self) -> bool { true }

    fn set_input_echo(&mut self, _value: bool) { /* TODO */ }
    fn set_input_immediate(&mut self, _value: bool) { /* TODO */ }
    fn set_output_protected(&mut self, _value: bool) { /* TODO */ }

    fn convert_to_stdio(&mut self) {
        self.is_stdio = true;
    }
}
