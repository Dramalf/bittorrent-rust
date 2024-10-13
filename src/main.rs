use std::{env, fs};
use serde_json;
mod parser;
mod sign;
mod metainfo_reader;
// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    match command.as_str() {
        "decode"=>{
            let encoded_value = &args[2];
            let decoded_value = parser::decode_bencoded_value(encoded_value);
            println!("{}", decoded_value.to_string());
        }
        "info"=>{
            let metainfo_file_path=&args[2];
            let metainfo_file_content=metainfo_reader::read_file_to_bytes(metainfo_file_path).unwrap();
            let parsed_value=parser::decode_bencoded_vec(&metainfo_file_content);
            println!("Tracker URL: {}",parsed_value["announce"].as_str().unwrap().trim_matches('"'));
            println!("Length: {:?}",parsed_value["info"]["length"].as_i64().unwrap());
        }
        _=>{
            println!("unknown command: {}", args[1]);
        }
    }
}
