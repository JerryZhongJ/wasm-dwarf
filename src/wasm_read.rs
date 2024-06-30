// Reads wasm file debug sections contents.

use std::collections::HashMap;
use std::io::Write;

use wasmparser::{Data, DataKind, Operator, Parser, Payload::*};

fn is_reloc_debug_section_name(name: &str) -> bool {
    return name.starts_with("reloc..debug_");
}

fn is_debug_section_name(name: &str) -> bool {
    return name.starts_with(".debug_");
}

fn is_linking_section_name(name: &str) -> bool {
    return name == "linking";
}

fn is_source_mapping_section_name(name: &str) -> bool {
    return name == "sourceMappingURL";
}

pub struct DebugSections<'a> {
    pub tables: HashMap<&'a str, Vec<u8>>,
    // pub tables_index: HashMap<usize, Vec<u8>>,
    pub reloc_tables: HashMap<&'a str, Vec<u8>>,
    pub linking: Option<Vec<u8>>,
    pub code_start: usize,
    pub func_offsets: Vec<usize>,
    pub data_segment_offsets: Vec<u32>,
}

impl<'a> DebugSections<'a> {
    pub fn read_sections(wasm: &'a [u8]) -> DebugSections<'a> {
        let parser = Parser::new(0);
        let mut linking: Option<Vec<u8>> = None;
        let mut tables = HashMap::new();
        // let mut tables_index = HashMap::new();
        let mut reloc_tables = HashMap::new();
        let mut code_start: usize = 0;
        let mut func_offsets = Vec::new();
        let mut data_segment_offsets = Vec::new();
        // let mut section_index = 0;
        for payload in parser.parse_all(wasm) {
            let payload = payload.unwrap();
            match payload {
                CustomSection(reader) => {
                    let name = reader.name();
                    let data = reader.data();
                    if is_debug_section_name(&name) {
                        tables.insert(name, data.to_vec());
                    } else if is_reloc_debug_section_name(&name) {
                        reloc_tables.insert(name, data.to_vec());
                    } else if is_linking_section_name(&name) {
                        linking = Some(data.to_vec());
                    } else if is_source_mapping_section_name(&name) {
                    }
                }
                CodeSectionStart { count, range, size } => {
                    code_start = range.start;
                }
                CodeSectionEntry(body) => {
                    let reader = body.get_binary_reader();

                    func_offsets.push(reader.original_position() - code_start);
                }
                ImportSection(reader) => func_offsets.push(0),
                DataSection(reader) => {
                    for data in reader.into_iter() {
                        let Data { kind, .. } = data.unwrap();
                        if let DataKind::Active { offset_expr, .. } = kind {
                            let mut op_reader = offset_expr.get_operators_reader();
                            let op = op_reader.read().unwrap();
                            if let Operator::I32Const { value } = op {
                                data_segment_offsets.push(value as u32);
                            } else {
                                panic!("Unexpected init expression operator");
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        DebugSections {
            tables,
            // tables_index,
            reloc_tables,
            linking,
            code_start,
            func_offsets,
            data_segment_offsets,
        }
    }
}

fn convert_to_leb(n: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut i = n;
    while i > 128 {
        buf.push(0x80 | (n & 0x7f) as u8);
        i = i >> 7;
    }
    buf.push(i as u8);
    buf
}

pub fn add_source_mapping_url_section(url: &str, write: &mut Write) {
    let name = b"sourceMappingURL";
    let mut result = Vec::new();
    let custom_section_id = convert_to_leb(0);
    result.extend_from_slice(&custom_section_id);
    let name_size = convert_to_leb(name.len());
    let url_size = convert_to_leb(url.len());
    let payload_size = convert_to_leb(name_size.len() + name.len() + url_size.len() + url.len());
    result.extend_from_slice(&payload_size);
    result.extend_from_slice(&name_size);
    result.extend_from_slice(name);
    result.extend_from_slice(&url_size);
    result.extend_from_slice(url.as_bytes());
    write.write(&result).expect("wasm result written");
}
