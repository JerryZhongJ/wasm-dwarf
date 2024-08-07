// Reads wasm file debug sections contents.

use std::collections::HashMap;
use std::io::Write;

use dwarf::DebugLocInfo;
use gimli::ReaderOffset;
use wasmparser::{Operator, Parser, Payload::*};
pub struct SourceMapEntry<'a> {
    pub address: usize,
    pub op: &'a str,
    pub source_file: &'a String,
    pub line: usize,
    pub source_code: &'a str,
}

pub fn map_source<'a>(
    wasm: &[u8],
    debug_info: &'a DebugLocInfo,
    sources_content: &'a Vec<Vec<String>>,
) -> Vec<SourceMapEntry<'a>> {
    let parser = Parser::new(0);

    let debug_line = &debug_info.locations;
    let mut index = 0;
    let mut source_map = Vec::new();
    let move_forward = |curr: usize, till: usize| {
        let mut index = curr;
        while index + 1 < debug_line.len() && till >= debug_line[index + 1].address as usize {
            index += 1;
        }
        return index;
    };
    let mut code_section_count = 0;
    for payload in parser.parse_all(wasm) {
        let payload = payload.unwrap();

        match payload {
            CodeSectionEntry(body) => 'walk_code_section: {
                let curr_section = code_section_count;
                code_section_count += 1;
                let reader = body.get_operators_reader().unwrap();
                let start_position = reader.original_position();
                let new_index = move_forward(index, start_position);
                // the start of a code section should map to a new debug line.
                if new_index == index {
                    println!("function ${}'s debug line is missing.", curr_section);
                    break 'walk_code_section;
                }

                index = new_index;
                // println!(
                //     "function ${} ({}:{})",
                //     curr_section,
                //     debug_info.sources[debug_line[index].source_id as usize],
                //     debug_line[index].line
                // );
                for pair in reader.into_iter_with_offsets() {
                    let (op, offset) = pair.unwrap();
                    //     while index + 1 < debug_line.len()
                    //         && offset >= debug_line[index + 1].address as usize
                    //     {
                    //         index += 1;

                    //         println!(
                    //             "{} {} {}:{}:{}",
                    //             offset,
                    //             debug_line[index].address,
                    //             debug_info.sources[debug_line[index].source_id as usize],
                    //             debug_line[index].line,
                    //             debug_line[index].column
                    //         );
                    //     }
                    index = move_forward(index, offset);
                    let op_name = match op {
                        Operator::Call { function_index } => Some("Call"),
                        Operator::CallIndirect {
                            type_index,
                            table_index,
                        } => Some("CallIndirect"),
                        _ => None,
                    };
                    if op_name == None {
                        continue;
                    }
                    let source_id = debug_line[index].source_id as usize;
                    let line = debug_line[index].line as usize;
                    // let column = debug_line[index].column as usize;
                    let source_content = &sources_content[source_id];
                    let source_code = source_content[line - 1].trim();

                    // println!(
                    //     "{}@{}\t\t{}({}:{})",
                    //     op_name.unwrap(),
                    //     offset,
                    //     source_code,
                    //     debug_info.sources[source_id],
                    //     line
                    // );
                    source_map.push(SourceMapEntry {
                        address: offset,
                        op: op_name.unwrap(),
                        line: line,
                        source_file: &debug_info.sources[source_id],
                        source_code: source_code,
                    });
                }
            }
            _ => {}
        }
    }
    // loop {
    //     let offset = parser.current_position();
    //     let state = parser.read_with_input(ParserInput::Default);
    //     if index + 1 < debug_line.len() {
    //         if offset >= debug_line[index + 1].address as usize {
    //             index += 1;
    //         }
    //     }
    //     match *state {
    //         ParserState::EndWasm => break,
    //         ParserState::Error(err) => panic!("Error: {:?}", err),
    //         ParserState::CodeOperator(ref op) => {
    //             let source_id = debug_line[index].source_id as usize;
    //             let source_content = &debug_info.sources_content[source_id];
    //             let line = debug_line[index].line as usize;
    //             match op {
    //                 Operator::Call { function_index } => {
    //                     source_map.push(SourceMapEntry {
    //                         address: offset,
    //                         op: "Call",
    //                         line: line,
    //                         source_file: &debug_info.sources[source_id],
    //                         source_code: &source_content[line - 1],
    //                     });
    //                 }
    //                 Operator::CallIndirect { index, table_index } => {
    //                     source_map.push(SourceMapEntry {
    //                         address: offset,
    //                         op: "CallIndirect",
    //                         line: line,
    //                         source_file: &debug_info.sources[source_id],
    //                         source_code: &source_content[line - 1],
    //                     });
    //                 }

    //                 _ => {}
    //             };
    //         }
    //         _ => {}
    //     }
    //}
    source_map
}
