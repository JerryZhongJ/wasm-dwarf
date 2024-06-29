// Reads wasm file debug sections contents.

use std::collections::HashMap;
use std::io::Write;

use dwarf::DebugLocInfo;
use gimli::ReaderOffset;
use wasmparser::{
    ImportSectionEntryType, Operator, Parser, ParserInput, ParserState, SectionCode, WasmDecoder,
};
pub struct SourceMapEntry<'a> {
    pub address: usize,
    pub op: &'a str,
    pub source_file: &'a String,
    pub line: usize,
    pub source_code: &'a String,
}

pub fn map_source<'a>(wasm: &[u8], debug_info: &'a DebugLocInfo) -> Vec<SourceMapEntry<'a>> {
    let mut parser = Parser::new(wasm);

    let debug_line = &debug_info.locations;
    let mut index = 0;
    let mut source_map = Vec::new();
    loop {
        let offset = parser.current_position();
        let state = parser.read_with_input(ParserInput::Default);
        if index + 1 < debug_line.len() {
            if offset >= debug_line[index + 1].address as usize {
                index += 1;
            }
        }
        match *state {
            ParserState::EndWasm => break,
            ParserState::Error(err) => panic!("Error: {:?}", err),
            ParserState::CodeOperator(ref op) => {
                let source_id = debug_line[index].source_id as usize;
                let source_content = &debug_info.sources_content[source_id];
                let line = debug_line[index].line as usize;
                match op {
                    Operator::Call { function_index } => {
                        source_map.push(SourceMapEntry {
                            address: offset,
                            op: "Call",
                            line: line,
                            source_file: &debug_info.sources[source_id],
                            source_code: &source_content[line - 1],
                        });
                    }
                    Operator::CallIndirect { index, table_index } => {
                        source_map.push(SourceMapEntry {
                            address: offset,
                            op: "CallIndirect",
                            line: line,
                            source_file: &debug_info.sources[source_id],
                            source_code: &source_content[line - 1],
                        });
                    }

                    _ => {}
                };
            }
            _ => {}
        }
    }
    source_map
}
