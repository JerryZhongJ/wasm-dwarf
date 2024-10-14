use std::fs::{canonicalize, File};
use std::io::prelude::*;
use std::{env, io::BufReader};

use dwarf::get_debug_loc;
use getopts::Options;
use map_source::map_source;
use reloc::reloc;
use std::path::Path;
use wasm_read::BinaryInfo;

extern crate getopts;
extern crate gimli;
extern crate rustc_serialize;
extern crate serde;
extern crate serde_json;
// extern crate vlq;
extern crate wasmparser;
mod dwarf;
mod map_source;
mod reloc;
mod wasm_read;

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

    let mut binary_info = BinaryInfo::read_sections(data.as_slice());

    if perform_reloc {
        if binary_info.linking.is_none() {
            panic!("relocation information was not found");
        }
        reloc(&mut binary_info);
    }
    let mut di = get_debug_loc(&binary_info);
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
            match canonicalize(path) {
                Ok(absolute_path) => {
                    source = Ok(absolute_path.to_str().unwrap().to_owned());
                    break;
                }
                Err(e) => {}
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

    let result = map_source(data.as_slice(), &binary_info, &di, &sources_content);

    let output = args.opt_str("o").unwrap();
    // let mut result = String::new();
    // for (id, path) in di.sources.iter().enumerate() {
    //     result += &format!("source {} {}\n", id, path);
    // }
    // for entry in source_map.iter() {
    //     let SourceMapEntry {
    //         address,
    //         op,
    //         source_file,
    //         line,
    //         source_code,
    //     } = entry;

    //     result += &format!(
    //         "{}@{}\t{}\t({}:{})\n",
    //         op, address, source_code, source_file, line
    //     )
    // }
    let mut f_out = File::create(output).expect("file cannot be created");
    f_out
        .write(
            serde_json::to_string(&result)
                .expect("Encoding error")
                .as_bytes(),
        )
        .expect("Writing file error");
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
