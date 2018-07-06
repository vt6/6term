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

use std::collections::hash_map::HashMap;
use std::sync::{Arc, Mutex};

use cairo;
use gtk::{self, WidgetExt};

use model;
use view;

pub struct Document {
    model: Arc<Mutex<model::Document>>,
    sections: HashMap<model::SectionID, view::Section>,
}

impl Document {
    pub fn new(model: Arc<Mutex<model::Document>>) -> Document {
        Document {
            model: model,
            sections: HashMap::new(),
        }
    }

    pub fn render(&mut self, canvas: &gtk::DrawingArea, ctx: &cairo::Context) {
        let model = self.model.lock().unwrap();
        let pixel_width = canvas.get_allocated_width();

        //draw background
        ctx.set_source_rgb(0., 0., 0.);
        ctx.paint();

        //draw sections
        ctx.set_source_rgb(1., 1., 1.);
        ctx.identity_matrix();

        let section_count = model.sections.len();
        for (idx, section) in model.sections.iter().enumerate() {
            let mut section_view = self.sections.entry(section.id()).or_insert_with(|| {
                view::Section::new(section, canvas)
            });
            let height = section_view.prepare_rendering(section, pixel_width);
            let show_cursor = idx == section_count - 1;
            section_view.render(section, ctx, show_cursor);
            ctx.translate(0., height as f64);
        }

        /* TODO kept for later reference
        let attr_list = pango::AttrList::new();
        let mut attr = pango::Attribute::new_strikethrough(true).unwrap();
        attr.set_start_index(6);
        attr.set_end_index(10);
        attr_list.insert(attr);
        layout.set_attributes(&attr_list);
        */
    }
}
