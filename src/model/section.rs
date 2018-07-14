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

pub enum CursorAction {
    Insert(String),
    //TODO replace "Char" by "GraphemeCluster" or sth like that
    DeletePreviousChar, //Backspace key
    DeleteNextChar,     //Delete key
    GotoPreviousChar,   //Left arrow key
    GotoNextChar,       //Right arrow key
}

#[derive(PartialEq,Eq)]
pub enum CursorActionResult {
    Unchanged,
    Changed,
    LineCompleted(String),
}

///Unique identifier for a section. This is a separate type to ensure that it is
///only generated by Document.make_section().
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct SectionID(u64);

impl SectionID {
    pub fn new() -> SectionID {
        SectionID(0)
    }

    pub fn incr(&mut self) {
        self.0 += 1;
    }
}

///A section is some amount of text that appears on screen, starting at the
///beginning of a line and ending at the end of a line.
pub struct Section {
    id: SectionID,
    text: String,
    ///Index into self.text where output from client programs will be appended. Everything before
    ///this cursor is client output, everything after this cursor is user input not yet submitted
    ///to a client program's stdin.
    output_cursor: usize,
    ///Index into self.text where user input is being inserted. This is always trailing the
    ///self.output_cursor (i.e., `self.output_cursor <= self.input_cursor`); see above for details.
    input_cursor: usize,
    ///This counter increases whenever this section is changed. It is used to
    ///indicate to the view when re-layouting is necessary.
    generation: u64,
}

impl Section {
    ///Use Document::make_section() instead.
    pub fn new(id: SectionID) -> Section {
        Section {
            id: id,
            text: String::new(),
            output_cursor: 0,
            input_cursor: 0,
            generation: 0,
        }
    }

    pub fn id(&self) -> SectionID {
        self.id
    }
    pub fn text(&self) -> &str {
        self.text.as_str()
    }
    pub fn input_cursor(&self) -> usize {
        self.input_cursor
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }

    ///Appends additional output to this section.
    pub fn append_output(&mut self, text: &str) {
        self.text.insert_str(self.output_cursor, text);
        let len = text.len();
        self.input_cursor += len;
        self.output_cursor += len;
        self.generation += 1;
    }

    ///Returns whether the text in this section has changed.
    pub fn execute_input_action(&mut self, action: CursorAction) -> CursorActionResult {
        let result = self.execute_input_action_priv(action);
        if result != CursorActionResult::Unchanged {
            self.generation += 1;
        }
        result
    }

    fn execute_input_action_priv(&mut self, action: CursorAction) -> CursorActionResult {
        use self::CursorAction::*;
        use self::CursorActionResult::*;
        match action {
            Insert(ref text) => {
                self.text.insert_str(self.input_cursor, text);
                self.input_cursor = self.input_cursor + text.len();
                if self.text.ends_with("\n") && self.input_cursor == self.text.len() {
                    let input = self.text.split_off(self.output_cursor);
                    self.input_cursor = self.output_cursor;
                    LineCompleted(input)
                } else {
                    Changed
                }
            },
            DeletePreviousChar | GotoPreviousChar => {
                if self.input_cursor <= self.output_cursor { return Unchanged; }
                //search for start of previous char
                self.input_cursor -= 1;
                while !self.text.is_char_boundary(self.input_cursor) {
                    self.input_cursor -= 1;
                }
                if let DeletePreviousChar = action {
                    self.text.remove(self.input_cursor);
                }
                Changed
            },
            DeleteNextChar => {
                if self.input_cursor == self.text.len() { return Unchanged; }
                self.text.remove(self.input_cursor); //cursor does not move
                Changed
            },
            GotoNextChar => {
                if self.input_cursor == self.text.len() { return Unchanged; }
                self.input_cursor += 1;
                while !self.text.is_char_boundary(self.input_cursor) {
                    self.input_cursor += 1;
                }
                Changed
            },
        }
    }
}
