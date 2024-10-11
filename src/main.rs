use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    decode_bencoded_start_at(encoded_value,0).0
}
fn decode_bencoded_start_at(raw_value:&str,start_index:usize)->(serde_json::Value,usize){
    let encoded_value=&raw_value[start_index..];
    // println!("index:{:?},len:{:?}",start_index,encoded_value);

    match encoded_value.chars().next().unwrap() {
        c if c.is_digit(10) => {
            // Example: "5:hello" -> "hello"
            let colon_index = encoded_value.find(':').unwrap();
            let number_string = &encoded_value[..colon_index];
            let number = number_string.parse::<i64>().unwrap();
            let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
            let part_len=colon_index  + number as usize;
            (serde_json::Value::String(string.to_string()),start_index+part_len+1)
        },
        'i' => {
            let end_index = encoded_value.find('e').unwrap();
            let number_string:&str=&encoded_value[1..end_index];
            let number:i64=number_string.parse::<i64>().unwrap();
            let real_number_str=number.to_string();
            if real_number_str.len()==number_string.len(){
                let part_len=number_string.len()+2; 
                (number.into(),start_index+part_len)
            }
            else{
                panic!("Unhandled encoded value: {}", encoded_value)
            } 
        },
        'l'=>{
            let mut list:Vec<serde_json::Value>=Vec::new();
            let mut index=start_index+1;

            while raw_value.chars().nth(index).unwrap()!='e'{
                let (value,new_index)=decode_bencoded_start_at(raw_value,index);
                list.push(value);
                index=new_index;
                if raw_value.len()<=index {
                    panic!("Unhandled encoded value: {}", encoded_value)
                }

            }
            (list.into(),index+1)
        }
        _ => {
            panic!("Unhandled encoded value: {}", encoded_value)
        }
    }

}
// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
