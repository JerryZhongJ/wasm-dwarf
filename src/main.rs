use std::fmt::Error;
use std::fs::File;
use std::io::prelude::*;
use std::{env, io::BufReader};

use dwarf::get_debug_loc;
use getopts::Options;
use map_source::{map_source, SourceMapEntry};
use reloc::reloc;
use std::path::Path;
use wasm_read::{add_source_mapping_url_section, DebugSections};

extern crate getopts;
extern crate gimli;
extern crate rustc_serialize;
extern crate vlq;
extern crate wasmparser;

mod dwarf;
mod map_source;
mod reloc;
mod wasm_read;

struct PrefixReplacements {
    replacements: Vec<(String, String)>,
}

impl PrefixReplacements {
    fn parse(input: &Vec<String>) -> PrefixReplacements {
        let mut replacements = Vec::new();
        for i in input.iter() {
            let separator = i.find('=');
            if let Some(separator_index) = separator {
                replacements.push((
                    i.chars().take(separator_index).collect(),
                    i.chars()
                        .skip(separator_index + 1)
                        .take(i.len() - 1 - separator_index)
                        .collect(),
                ));
            } else {
                replacements.push((i.clone(), String::new()))
            }
        }
        return PrefixReplacements { replacements };
    }

    fn replace(&self, path: &String) -> String {
        let mut result = path.clone();
        for (ref old_prefix, ref new_prefix) in self.replacements.iter() {
            if path.starts_with(old_prefix) {
                result = result.split_off(old_prefix.len());
                result.insert_str(0, new_prefix);
                return result;
            }
        }
        result
    }

    fn replace_all(&self, paths: &mut Vec<String>) {
        for path in paths.iter_mut() {
            *path = self.replace(&path);
        }
    }
}

fn main() {
    let mut opts = Options::new();
    opts.optopt("o", "", "set output file name", "NAME");
    opts.optflag("", "relocation", "perform relocation first");
    opts.optflag("l", "list-source", "list source files");
    opts.optopt(
        "m",
        "source-map",
        "specifies sourceMappingURL section contest",
        "URL",
    );
    opts.optmulti(
        "s",
        "source-roots",
        "Search source files under these roots if they are not found under current directory.",
        "DIR",
    );
    opts.optflag("h", "help", "print this help menu");

    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();
    let args = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if args.opt_present("h")
        || args.free.len() < 1
        || !(args.opt_present("o") || args.opt_present("l"))
    {
        return print_usage(&program, opts);
    }

    let perform_reloc = args.opt_present("relocation");
    let filename = args.free[0].clone();
    let mut f = File::open(filename).expect("file not found");
    let mut data = Vec::new();
    f.read_to_end(&mut data).expect("unable to read file");

    let mut debug_sections = DebugSections::read_sections(data.as_slice());

    if perform_reloc {
        if debug_sections.linking.is_none() {
            panic!("relocation information was not found");
        }
        reloc(&mut debug_sections);
    }
    let mut di = get_debug_loc(&debug_sections);
    // for debug_info in di.locations.iter() {
    //     println!(
    //         "{} {} {} {}",
    //         debug_info.address, debug_info.source_id, debug_info.line, debug_info.column
    //     );
    // }
    let mut source_roots = args.opt_strs("source-roots");
    source_roots.insert(0, String::from("."));
    let _sources = di.sources;
    di.sources = Vec::new();
    for file in _sources.iter() {
        let mut source = Err(format!("source file {} not found", file));
        for source_root in source_roots.iter() {
            let path = Path::new(source_root).join(file);
            if path.as_path().exists() {
                source = Ok(path.to_str().unwrap().to_owned());
                break;
            }
        }
        di.sources.push(source.expect(""));
    }

    if args.opt_present("l") {
        for file in di.sources.iter() {
            println!("{}", file);
        }
        return;
    }
    let mut sources_content = Vec::new();
    for file in di.sources.iter() {
        let f = File::open(file).expect("file not found");
        let f = BufReader::new(f);
        sources_content.push(f.lines().filter_map(|line| line.ok()).collect());
    }

    let source_map = map_source(data.as_slice(), &di, &sources_content);

    let output = args.opt_str("o").unwrap();
    let mut result = String::new();
    for (id, path) in di.sources.iter().enumerate() {
        result += &format!("source {} {}\n", id, path);
    }
    for entry in source_map.iter() {
        let SourceMapEntry {
            address,
            op,
            source_file,
            line,
            source_code,
        } = entry;

        result += &format!(
            "{}@{}\t{}\t({}:{})\n",
            op, address, source_code, source_file, line
        )
    }
    let mut f_out = File::create(output).expect("file cannot be created");
    f_out.write(result.as_bytes()).expect("data written");
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] <INPUT>", program);
    print!("{}", opts.usage(&brief));
    println!(
        "
Reading DWARF data from the wasm object files, and converting to source maps.

Usage:

    # Read and convert to JSON
    wasm-dwarf foo.wasm -o foo.map
"
    );
}
