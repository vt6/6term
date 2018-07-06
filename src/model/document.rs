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

use std::sync::{Arc, Mutex};

use model;

///This is the main model object that both the GUI thread and the Tokio
///eventloop have access to.
pub struct Document {
    pub sections: Vec<model::Section>,
    next_section_id: model::SectionID,
}

impl Document {
    pub fn new() -> Arc<Mutex<Document>> {
        Arc::new(Mutex::new(Document {
            sections: Vec::new(),
            next_section_id: model::SectionID::new(),
        }))
    }

    pub fn make_section(&mut self, text: String) -> model::Section {
        self.next_section_id.incr();
        model::Section::new(text, self.next_section_id)
    }
}
