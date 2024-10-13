use std::fs::File;
use std::io::{self, Read};

// 将读取文件为字节数组的函数定义在这里
pub fn read_file_to_bytes(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(file_path)?;  // 打开文件
    let mut buffer = Vec::new();            // 创建一个空的 Vec<u8>
    file.read_to_end(&mut buffer)?;         // 将文件内容读取到 buffer 中
    Ok(buffer)                              // 返回字节数组
}
