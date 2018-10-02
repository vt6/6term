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

use vt6::server as vt6s;
use vt6::common::core as vt6c;

pub struct TermHandler<H> {
    pub inner: H,
}

impl<C: vt6s::Connection + TermConnection, H: vt6s::Handler<C>> vt6s::Handler<C> for TermHandler<H> {
    fn handle(&self, msg: &vt6c::msg::Message, conn: &mut C, send_buffer: &mut [u8]) -> Option<usize> {
        //TODO move into vt6::server::core::Handler
        let has_core1 = conn.is_module_enabled("core").map_or(false, |version| version.major == 1);
        if msg.type_name() == ("core", "make-stdio") && has_core1{
            conn.convert_to_stdio();
            vt6c::msg::MessageFormatter::new(send_buffer, "core.is_stdio", 0)
                .finalize().ok()
        } else {
            self.inner.handle(msg, conn, send_buffer)
        }
    }

    fn can_use_module(&self, name: &str, major_version: u16, conn: &C) -> Option<u16> {
        if name == "term" && major_version == 1 {
            if conn.is_module_enabled("core").is_some() {
                Some(0)
            } else {
                None
            }
        } else {
            self.inner.can_use_module(name, major_version, conn)
        }
    }

    fn handle_property<'c>(&self, name: &str, requested_value: Option<&[u8]>, conn: &mut C, send_buffer: &mut [u8]) -> Option<usize> {
        use vt6::common::core::msg::prerecorded::publish_property;
        use vt6::common::core::DecodeArgument;
        match name {
            "term.input-echo" => {
                if let Some(value) = requested_value.and_then(bool::decode) {
                    conn.set_input_echo(value);
                }
                publish_property(send_buffer, name, &conn.is_input_echo())
            },
            "term.input-immediate" => {
                if let Some(value) = requested_value.and_then(bool::decode) {
                    conn.set_input_immediate(value);
                }
                publish_property(send_buffer, name, &conn.is_input_immediate())
            },
            "term.output-protected" => {
                if let Some(value) = requested_value.and_then(bool::decode) {
                    conn.set_output_protected(value);
                }
                publish_property(send_buffer, name, &conn.is_output_protected())
            },
            "term.output-reflow" => {
                publish_property(send_buffer, name, &conn.is_output_reflow())
            },
            "term.output-wordwrap" => {
                publish_property(send_buffer, name, &conn.is_output_wordwrap())
            },
            _ => self.inner.handle_property(name, requested_value, conn, send_buffer),
        }
    }
}

pub trait TermConnection: vt6s::Connection {
    fn is_input_echo(&self) -> bool;
    fn is_input_immediate(&self) -> bool;
    fn is_output_protected(&self) -> bool;
    fn is_output_reflow(&self) -> bool;
    fn is_output_wordwrap(&self) -> bool;

    fn set_input_echo(&mut self, value: bool);
    fn set_input_immediate(&mut self, value: bool);
    fn set_output_protected(&mut self, value: bool);

    fn convert_to_stdio(&mut self);
}
