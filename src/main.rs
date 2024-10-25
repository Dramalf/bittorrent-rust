use std::fs;
use bittorrent_starter_rust::{peer::MessageFramer, tracker::*};
use bittorrent_starter_rust::torrent::Torrent;
use bittorrent_starter_rust::peer::*;
use std::path::PathBuf;
use clap::{command, Parser, Subcommand};
mod torrent;
mod my_parser;
mod metainfo_reader;
use anyhow::Context;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use futures_util::{SinkExt, StreamExt};
use serde_bencode;
use serde_json;
use sha1::{Digest, Sha1};
use std::net::SocketAddrV4;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum Command{
    Decode{
        encoded_value:String,
    },
    Info{
        torren:PathBuf,
    },
    Peers{
        torrent:PathBuf,
    },
    Handshake{
        torrent:PathBuf,
        addr:String,
    },
    DownloadPiece{
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
        piece: usize,
    }
}

const BLOCK_SIZE: usize=1<<14;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.command{
        Command::Decode{encoded_value}=>{
            let decoded_value = my_parser::decode_bencoded_value(&encoded_value);
            println!("{}", decoded_value.to_string());
        }
        Command::Info { torren }=>{
            let f: Vec<u8>=fs::read(torren).context("read torrent file")?;
            let t:Torrent=serde_bencode::from_bytes(&f).context("parse torrent file")?;
            // println!("{:?}", t);
            println!("Tracker URL: {}", t.announce);
            let length=t.info.length;
            println!("Length: {length}");
            let info_hash = t.info_hash();
            println!("Info Hash: {}", hex::encode(&info_hash));
            println!("Piece Length: {}", t.info.piece_length);
            println!("Piece Hashes:");
            for hash in t.info.pieces.0 {
                println!("{}", hex::encode(&hash));
            }
        }
        Command::Peers { torrent }=>{
            let f: Vec<u8>=fs::read(torrent).context("read torrent file")?;
            let t:Torrent=serde_bencode::from_bytes(&f).context("parse torrent file")?;
            let tracker:TrackerRequest=TrackerRequest::new(&t);
            let response:TrackerResponse=tracker.send(&t.announce,&t.info_hash()).await?;
            for peer in response.peers.0{
                println!("{}", peer);
            }
        },
        Command::Handshake { torrent, addr}=>{
            let f: Vec<u8>=fs::read(torrent).context("read torrent file")?;
            let t:Torrent=serde_bencode::from_bytes(&f).context("parse torrent file")?;
            let info_hash=t.info_hash();
            let mut handshake=Handshake::new(info_hash, *b"-0-1-2-3-4-5-6-7-8-9");
            let mut stream = TcpStream::connect(&addr).await?;
            let bytes=handshake.to_btyes();
            stream.write_all(&bytes).await?;
            let mut buf=[0;68];
            stream.read_exact(&mut buf).await?;
            handshake=Handshake::from_bytes(&buf);
            // assert_eq!(handshake.length, 19);
            // assert_eq!(&handshake.bittorrent, b"BitTorrent protocol");
            println!("Peer ID: {}", hex::encode(&handshake.peer_id));
        }   
        Command::DownloadPiece {
            output,
            torrent,
            piece: piece_i,
        } =>{
            let f: Vec<u8>=fs::read(torrent).context("read torrent file")?;
            let t:Torrent=serde_bencode::from_bytes(&f).context("parse torrent file")?;
            let length=t.info.length;
            let info_hash=t.info_hash();
            let tracker:TrackerRequest=TrackerRequest::new(&t);
            let response:TrackerResponse=tracker.send(&t.announce,&t.info_hash()).await?;
            let addr=&response.peers.0[0];
            let mut stream = tokio::net::TcpStream::connect(addr).await.context("connect to peer")?;
            let mut handshake = Handshake::new(info_hash, *b"00112233445566778899");
            let bytes=handshake.to_btyes();
            stream.write_all(&bytes).await?;
            let mut buf=[0;68];
            stream.read_exact(&mut buf).await?;

            let mut peer: tokio_util::codec::Framed<TcpStream, MessageFramer> = tokio_util::codec::Framed::new(stream, MessageFramer);
            let bitfield=peer.next().await.expect("peer always sends abitfields").context("peer messagewasinvalid")?;
            eprintln!("{:?}",bitfield);
            peer.send(Message {
                id: MessageId::Interested,
                payload: Box::new(Vec::new()),
            }).await.context("send interested message")?;
            let unchoke = peer
                .next()
                .await
                .expect("peer always sends an unchoke")
                .context("peer message was invalid")?;
            assert_eq!(unchoke.id, MessageId::Unchoke);
            assert!(unchoke.payload.is_empty());
            eprintln!("unchoke message:{:?}",unchoke);
            let piece_hash = &t.info.pieces.0[piece_i];
            let piece_size = if piece_i == t.info.pieces.0.len() - 1 {
                let md = length as usize % t.info.piece_length ;
                if md == 0 {
                    t.info.piece_length
                } else {
                    md 
                }
            } else {
                t.info.piece_length
            };
            eprintln!("t.info:{:?}",t.info);
            let mut block_result:Vec<u8>=Vec::with_capacity(piece_size);
            let nblocks = (piece_size + (BLOCK_SIZE - 1)) / BLOCK_SIZE;
            for block in 0..nblocks {
                let block_size = if block == nblocks - 1 {
                    let md = piece_size % BLOCK_SIZE;
                    if md == 0 {
                        BLOCK_SIZE
                    } else {
                        md
                    }
                } else {
                    BLOCK_SIZE
                };
                
                // let  payload:Vec<u8>=[(piece_i as u32).to_be_bytes().as_slice(),(begin as i32).to_be_bytes().as_slice(),(block_size as i32).to_be_bytes().as_slice()].concat();
                // eprintln!("payload:{:?}",payload);
                let mut request = Request::new(
                    piece_i as u32,
                    (block * BLOCK_SIZE) as u32,
                    block_size as u32,
                );
                let request_bytes = Vec::from(request.as_bytes_mut());
                // eprint!("request_bytes:{:?}",request_bytes);
                let request=Message{id:MessageId::Request,payload:Box::new(request_bytes)};
                // eprintln!("{} send:{:?}",begin,request);

                peer.send(request).await.context("send request message from fail")?;
                let response = peer
                .next()
                .await
                .expect("peer always sends an bad response")
                .context("peer message was invalid")?;
                // eprint!("response:{:?}",response);
                anyhow::ensure!(response.id == MessageId::Piece);
                block_result.extend_from_slice(&response.payload[8..]);

                // begin+=block_size;
                // remain-=block_size;
            }
            let mut hasher = Sha1::new();
            hasher.update(&block_result);
            let hash: [u8; 20] = hasher
                .finalize()
                .try_into()
                .expect("GenericArray<_, 20> == [_; 20]");
            assert_eq!(&hash, piece_hash);

            tokio::fs::write(&output, block_result)
                .await
                .context("write out downloaded piece")?;
            // println!("Piece {piece_i} downloaded to {}.", output.display());

        }
        _=>{
            println!("unknown command");
        }
    }
    Ok(())
}

#[repr(C)]
#[repr(packed)]
pub struct Request {
    index: [u8; 4],
    begin: [u8; 4],
    length: [u8; 4],
}

impl Request {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        eprintln!("{}length{:?}",length,length.to_be_bytes());
        Self {
            index: index.to_be_bytes(),
            begin: begin.to_be_bytes(),
            length: length.to_be_bytes(),
        }
    }

    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn length(&self) -> u32 {
        u32::from_be_bytes(self.length)
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let bytes = self as *mut Self as *mut [u8; std::mem::size_of::<Self>()];
        // Safety: Self is a POD with repr(c) and repr(packed)
        let bytes: &mut [u8; std::mem::size_of::<Self>()] = unsafe { &mut *bytes };
        bytes
    }
}