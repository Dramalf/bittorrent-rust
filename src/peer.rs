use anyhow::Context;
use bytes::{Buf, BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddrV4;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use tokio_util::codec::Framed;

#[derive(Debug)]
pub struct Handshake {
    pub length: u8,
    pub bittorrent: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}
impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            length: 19,
            bittorrent: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }
    pub fn to_btyes(&self) -> [u8; 68] {
        let mut bytes = [0; 68];
        bytes[0] = self.length;
        bytes[1..20].copy_from_slice(&self.bittorrent);
        bytes[28..48].copy_from_slice(&self.info_hash);
        bytes[48..68].copy_from_slice(&self.peer_id);
        bytes
    }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            length: bytes[0],
            bittorrent: bytes[1..20].try_into().unwrap(),
            reserved: bytes[20..28].try_into().unwrap(),
            info_hash: bytes[28..48].try_into().unwrap(),
            peer_id: bytes[48..68].try_into().unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 9,
}

#[derive(Debug, Clone)]

pub struct Message {
    pub id: MessageId,
    pub payload: Box<Vec<u8>>,
}

pub struct MessageFramer;

const MAX: usize = 1 << 16;

impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Not enough data to read length marker.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;
        eprintln!("{},length_bytes:{:?}", length, length_bytes);
   
        if length == 0 {
            // this is a heartbeat message.
            // discard it.
            src.advance(4);
            // and then try again in case the buffer has more messages
            return self.decode(src);
        }
        if src.len() < 5 {
            return Ok(None);
        }
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }
        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.
        let id = match src[4] {
            0 => MessageId::Choke,
            1 => MessageId::Unchoke,
            2 => MessageId::Interested,
            3 => MessageId::NotInterested,
            4 => MessageId::Have,
            5 => MessageId::Bitfield,
            6 => MessageId::Request,
            7 => MessageId::Piece,
            9 => MessageId::Cancel,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid message id",
                ))
            }
        };
        let data = if src.len() > 5 {
            src[5..4 + length].to_vec()
        } else {
            Vec::new()
        };
        src.advance(4 + length);

        // Convert the data to a string, or fail if it is not valid utf-8.
        Ok(Some(Message {
            id,
            payload: Box::new(data),
        }))
    }
}

impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send a message if it is longer than the other end will
        // accept.
        if item.payload.len() + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Encode Frame of length {} is too large.", item.payload.len()),
            ));
        }

        // Convert the length into a byte array.
        let len_slice = u32::to_be_bytes(item.payload.len() as u32 + 1);

        // Reserve space in the buffer.
        dst.reserve(4 /* length */ + 1 /* tag */ + item.payload.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.put_u8(item.id as u8);
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}




#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::Torrent;
    use anyhow::Context;
    use std::fs;
    #[test]
    fn test_bytes_mut() -> Result<(), Box<dyn std::error::Error>> {
        let f: Vec<u8> = fs::read("sample.torrent").context("read torrent file")?;
        let t: Torrent = serde_bencode::from_bytes(&f).context("parse torrent file")?;
        let info_hash = t.info_hash();
        let handshake = Handshake::new(info_hash, *b"-0-1-2-3-4-5-6-7-8-9");
        let bytes = handshake.to_btyes();
        println!("{:?}", bytes);
        assert_eq!(bytes.len(), 68);
        Ok(())
    }
}
