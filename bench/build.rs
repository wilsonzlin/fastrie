use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Entity {
    codepoints: Vec<u32>,
    characters: String,
}

fn create_string_literal(s: &str) -> String {
    format!("\"{}\"", s
        .chars()
        .map(|b| if b >= ' ' && b <= '~' && b != '\\' && b != '"' {
            b.to_string()
        } else {
            format!("\\u{{{:02x}}}", b as u32)
        })
        .collect::<String>())
}

fn main() {
    let file = File::open("entities.json").unwrap();
    let entities: HashMap<String, Entity> = serde_json::from_reader(file).unwrap();

    let mut code = String::new();
    code.push_str("static STATIC_MAP: phf::Map<&'static [u8], &'static str> = phf::phf_map! {\n");
    for (rep, entity) in entities {
        code.push_str(format!("\tb\"{}\" => {},\n", rep, create_string_literal(entity.characters.as_str())).as_str());
    };
    code.push_str("};\n");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("phf_map.rs");
    let mut dest_file = File::create(&dest_path).unwrap();
    dest_file.write_all(code.as_bytes()).unwrap();
}
