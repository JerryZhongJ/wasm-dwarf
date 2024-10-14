// Reads wasm file debug sections contents.

use dwarf::DebugLocInfo;
use serde::Serialize;

use wasmparser::{Operator, Parser, Payload::*};

use crate::wasm_read::BinaryInfo;
// pub struct SourceMapEntry<'a> {
//     pub address: usize,
//     pub op: &'a str,
//     pub source_file: &'a String,
//     pub line: usize,
//     pub source_code: &'a str,
// }

#[derive(Serialize)]
pub struct OutputResult {
    pub source_files: Vec<String>,
    pub import_num: usize,
    pub function_def: Vec<Option<(u32, u32)>>,
    pub call_indirect: Vec<Option<(u32, u32, u32)>>,
}
pub fn map_source(
    wasm: &[u8],
    binary_info: &BinaryInfo,
    debug_info: &DebugLocInfo,
    sources_content: &Vec<Vec<String>>,
) -> OutputResult {
    let parser = Parser::new(0);

    let debug_line = &debug_info.locations;
    let mut index = 0;
    let mut function_def = Vec::new();
    let mut call_indirect = Vec::new();
    // let mut source_map = Vec::new();
    let move_forward = |curr: usize, till: usize| {
        let mut index = curr;
        while index + 1 < debug_line.len() && till >= debug_line[index + 1].address as usize {
            index += 1;
        }

        // for i in (curr + 1)..index {
        //     println!(
        //         "Debug line@{} ({},{},{}) is ignored, current @ {}.",
        //         debug_line[i].address,
        //         debug_line[i].source_id,
        //         debug_line[i].line,
        //         debug_line[i].column,
        //         till
        //     );
        // }

        return index;
    };
    let mut code_section_count = 0;
    for payload in parser.parse_all(wasm) {
        let payload = payload.unwrap();

        match payload {
            CodeSectionEntry(body) => {
                let func_indice = code_section_count + binary_info.import_func_num as usize;
                code_section_count += 1;
                let reader = body.get_operators_reader().unwrap();
                let start_position = reader.original_position();
                let new_index = move_forward(index, start_position);
                let mut skip = false;
                let mut skip_call = 0;
                let mut skip_call_indirect = 0;
                // the start of a code section should map to a new debug line.
                let func_name = if binary_info.function_names.contains_key(&func_indice) {
                    binary_info.function_names[&func_indice].to_owned()
                } else {
                    format!("${}", func_indice)
                };

                if (new_index > 0 && new_index == index)
                    || (new_index == 0 && debug_line[0].address as usize > start_position)
                {
                    // println!("function {}'s debug line is missing.", func_name);
                    skip = true;
                    function_def.push(None)
                } else {
                    function_def.push(Some((
                        debug_line[new_index].source_id,
                        debug_line[new_index].line,
                    )));
                }

                index = new_index;
                // println!(
                //     "function {}@{} ({}:{})",
                //     func_name,
                //     start_position,
                //     debug_line[index].source_id as usize,
                //     debug_line[index].line
                // );

                for pair in reader.into_iter_with_offsets() {
                    let (op, offset) = pair.unwrap();

                    index = move_forward(index, offset);
                    match op {
                        Operator::CallIndirect {
                            type_index,
                            table_index,
                        } => {
                            if skip {
                                skip_call_indirect += 1;
                                call_indirect.push(None);
                            } else {
                                call_indirect.push(Some((
                                    debug_line[index].source_id,
                                    debug_line[index].line,
                                    debug_line[index].column,
                                )));
                            }
                        }
                        _ => continue,
                    };

                    // let source_id = debug_line[index].source_id as usize;
                    // let line = debug_line[index].line as usize;
                    // // let column = debug_line[index].column as usize;
                    // let source_content = &sources_content[source_id];
                    // let source_code = source_content[line - 1].trim();

                    // // println!(
                    // //     "{}@{}\t\t{}({}:{})",
                    // //     op_name.unwrap(),
                    // //     offset,
                    // //     source_code,
                    // //     debug_info.sources[source_id],
                    // //     line
                    // // );
                    // source_map.push(SourceMapEntry {
                    //     address: offset,
                    //     op: op_name.unwrap(),
                    //     line: line,
                    //     source_file: &debug_info.sources[source_id],
                    //     source_code: source_code,
                    // });
                }
                if skip {
                    println!(
                        "function \x1b[34m{}\x1b[0m has \x1b[33m{}\x1b[0m Call and \x1b[33m{}\x1b[0m CallIndirect skipped.",
                        func_name, skip_call, skip_call_indirect
                    );
                }
            }
            _ => {}
        }
    }

    OutputResult {
        source_files: debug_info.sources.clone(),
        import_num: binary_info.import_func_num,
        function_def: function_def,
        call_indirect: call_indirect,
    }
}
