use crate::torrent::Torrent;
use anyhow::Context;
use peers::Peers;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    /// The info hash of the torrent
    // pub info_hash: [String],
    /// A unique identifier for your client
    /// 20 bytes long, will need to be URL encoded
    /// Note: this is NOT the hexadecimal representation, which is 40 bytes long
    pub peer_id: String,
    /// The port your client is listening on
    pub port: u16,
    /// The total amount uploaded so far
    pub uploaded: u64,
    /// The total amount downloaded so far
    pub downloaded: u64,
    /// whether the peer list should use the compact representation
    /// For the purposes of this challenge, set this to 1.
    /// The compact representation is more commonly used in the wild, the non-compact representation is mostly supported for backward-compatibility.
    pub compact: u8,
    pub left: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker.
    /// You can ignore this value for the purposes of this challenge.
    pub interval: u64,
    /// A string, which contains list of peers that your client can connect to.
    /// Each peer is represented using 6 bytes.
    ///  The first 4 bytes are the peer's IP address
    ///  and the last 2 bytes are the peer's port number.
    pub peers: Peers,
}

impl TrackerRequest {
    pub fn new(t: &Torrent) -> Self {
        Self {
            // info_hash: t.info_hash(),
            peer_id: "-0-1-2-3-4-5-6-7-8-9".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            compact: 1,
            left: t.info.length,
        }
    }
    pub fn gen_url(&self, url: &String,info_hash:&[u8;20]) -> anyhow::Result<String> {
        let params = serde_urlencoded::to_string(&self).context("url encode request parameters")?;
        let encoded_info_hash = urlencode(info_hash);
        Ok(format!("{}?{}&info_hash={}", url, params,encoded_info_hash))
    }
    pub async fn send(&self, url: &String,info_hash:&[u8;20]) -> anyhow::Result<TrackerResponse> {
        let tracker_url = self.gen_url(url,info_hash)?;
        let response = reqwest::get(tracker_url).await.context("query tracker")?;
        let response = response.bytes().await.context("fetch tracker response")?;
        let response: TrackerResponse =
            serde_bencode::from_bytes(&response).context("parse tracker response")?;
        Ok(response)
    }
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

mod peers {
    use serde::de::{self, Deserialize, Deserializer, Visitor};
    use serde::ser::{Serialize, Serializer};
    use std::fmt;
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[derive(Debug, Clone)]
    pub struct Peers(pub Vec<SocketAddrV4>);
    struct PeersVisitor;
    impl<'de> Visitor<'de> for PeersVisitor {
        type Value = Peers;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("6 bytes, the first 4 bytes are a peer's IP address and the last 2 are a peer's port number")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() % 6 != 0 {
                return Err(E::custom(format!("Invalid length of {}", v.len())));
            }
            Ok(Peers(
                v.chunks(6)
                    .map(|slice_6| {
                        let ip = Ipv4Addr::new(slice_6[0], slice_6[1], slice_6[2], slice_6[3]);
                        let port = u16::from_be_bytes([slice_6[4], slice_6[5]]);
                        SocketAddrV4::new(ip, port)
                    })
                    .collect(),
            ))
        }
    }
    impl<'de> Deserialize<'de> for Peers {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_bytes(PeersVisitor)
        }
    }
    impl Serialize for Peers {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut v = Vec::with_capacity(self.0.len() * 6);
            for peer in &self.0 {
                v.extend_from_slice(&peer.ip().octets());
                v.extend_from_slice(&peer.port().to_be_bytes());
            }
            serializer.serialize_bytes(&v)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::Torrent;
    use crate::tracker::TrackerRequest;
    use serde_bencode::from_bytes;
    use std::fs;

    #[test]
    fn test_tracker_request()->anyhow::Result<()> {
        let f: Vec<u8> = fs::read("sample.torrent")
            .context("read torrent file")
            .unwrap();
        let t: Torrent = from_bytes(&f).context("parse torrent file").unwrap();

        let tracker_request = TrackerRequest::new(&t);
        let  url=tracker_request.gen_url(&t.announce,&t.info_hash())?;
        // let a=tracker_request.send(&t.announce);
        // println!("{:?}", a);
        Ok(assert_eq!(url.as_str(),"http://bittorrent-test-tracker.codecrafters.io/announce?peer_id=-0-1-2-3-4-5-6-7-8-9&port=6881&uploaded=0&downloaded=0&compact=1&info_hash=%d6%9f%91%e6%b2%ae%4c%54%24%68%d1%07%3a%71%d4%ea%13%87%9a%7f"))
    }
}
