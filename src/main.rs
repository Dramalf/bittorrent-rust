use std::fs;
use bittorrent_starter_rust::tracker::*;
use bittorrent_starter_rust::torrent::Torrent;
use bittorrent_starter_rust::peer::Handshake;
use std::path::PathBuf;
use clap::{command, Parser, Subcommand};
mod torrent;
mod my_parser;
mod metainfo_reader;
use anyhow::Context;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

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
    }
}




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
        _=>{
            println!("unknown command");
        }
    }
    Ok(())
}
