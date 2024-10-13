use std::{result, string};
use sha1::{Sha1, Digest};

use serde_json;
use crate::sign::Sign;
pub fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    decode_bencoded_start_at(encoded_value,0).0
}
fn decode_bencoded_start_at(raw_value:&str,start_index:usize)->(serde_json::Value,usize){
    let encoded_value=&raw_value[start_index..];
    eprintln!("index:{:?},len:{:?}",start_index,encoded_value);

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
            while raw_value.len()>index && raw_value.chars().nth(index).unwrap()!='e'{
                let (value,new_index)=decode_bencoded_start_at(raw_value,index);
                list.push(value);
                index=new_index;

            }
            (list.into(),index+1)
        }
        'd'=>{
            let mut dict:serde_json::Map<String,serde_json::Value>=serde_json::Map::new();
            let mut index=start_index+1;
            while  raw_value.len()>index&&raw_value.chars().nth(index).unwrap()!='e'{
                let (key,new_index)=decode_bencoded_start_at(raw_value,index);

                let (value,new_index)=decode_bencoded_start_at(raw_value,new_index);
                if let serde_json::Value::String(key)=key{
                    dict.insert(key,value);
                }
                else{
                    panic!("Unhandled encoded value: {}", encoded_value)
                }
                index=new_index;
            }
            (dict.into(),index+1)
        }
        _ => {
            panic!("Unhandled encoded value: {}", encoded_value)
        }
    }

}

pub fn decode_bencoded_vec(encoded_vec:&Vec<u8>)->serde_json::Value{
    decode_bencoded_vec_start_at(&encoded_vec,0).0
}

pub fn decode_bencoded_vec_start_at(raw_vec:&[u8],start_index:usize)->(serde_json::Value,usize){
    let encoded_vec=&raw_vec[start_index..];
    // eprintln!("index:{:?},len:{:?}",start_index,encoded_vec);

    match encoded_vec.iter().next().unwrap() {
        &n if  48<=n && n<=57 => {
            // Example: "5:hello" -> "hello"
            let colon_index = encoded_vec.iter().position(|&x|x==Sign::colon).unwrap();
            match read_vecu8_to_string(&encoded_vec[0..colon_index]){
                Some(number_string)=> {
                    let number = number_string.parse::<i64>().unwrap();
                    let read_result: Option<String> =read_vecu8_to_string(&encoded_vec[colon_index + 1..colon_index + 1 + number as usize]);
                    if let Some(string)=read_result{
                        let part_len=colon_index + number as usize;
                        (serde_json::Value::String(string),start_index+part_len+1)
                    }
                    else{
                        (encoded_vec[0..colon_index].to_vec().into(),start_index+colon_index+number as usize+1)
                    }
                }
                _=>{
                    panic!("Can not read length of the string at index: {}", start_index);
                }
            }
        },
        &Sign::i => {
            let end_index = encoded_vec.iter().position(|&x|x==Sign::e).unwrap();
            let number_string=read_vecu8_to_string(&encoded_vec[1..end_index]).unwrap();
            let number:i64=number_string.parse::<i64>().unwrap();
            let real_number_str=number.to_string();
            if real_number_str.len()==number_string.len(){
                let part_len=number_string.len()+2; 
                (number.into(),start_index+part_len)
            }
            else{
                panic!("Unhandled encoded value at index: {}", start_index);
            } 
        },
        &Sign::l=>{
            let mut list:Vec<serde_json::Value>=Vec::new();
            let mut index=start_index+1;
            while raw_vec.len()>index && raw_vec[index]!=Sign::e{
                let (value,new_index)=decode_bencoded_vec_start_at(&raw_vec,index);
                list.push(value);
                index=new_index;

            }
            (list.into(),index+1)
        }
        &Sign::d=>{
            let mut dict:serde_json::Map<String,serde_json::Value>=serde_json::Map::new();
            let mut index=start_index+1;
            while  raw_vec.len()>index&&raw_vec[index]!=Sign::e{
                let (key,new_index)=decode_bencoded_vec_start_at(raw_vec,index);

                let (value,new_index)=decode_bencoded_vec_start_at(raw_vec,new_index);
                if let serde_json::Value::String(key)=key{
                    dict.insert(key,value);
                }
                else{
                    panic!("Unhandled encoded value at: {}", start_index)
                }
                index=new_index;
            }
            (dict.into(),index+1)
        }
        _ => {
            panic!("Unhandled encoded value at: {}", start_index)
        }
    }

}

fn read_vecu8_to_string(vec:&[u8])->Option<String>{
    match String::from_utf8(vec.to_vec()){
        Ok(result)=>Some(result),
        Err(_)=>{
            let mut hasher = Sha1::new();
            hasher.update(vec);
            match hasher.finalize(){
                slice=>{
                    Some(hex::encode(&slice))
                }
                _=>None
            }
        }
    }
}