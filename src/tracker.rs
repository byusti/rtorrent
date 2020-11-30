extern crate hex;
extern crate serde_bencode;
extern crate serde_bytes;

use crate::peers::*;
use crate::torrentfile::TorrentFile;
#[allow(unused_imports)]
use serde_bencode::{de, ser};
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Error;
#[allow(unused_imports)]
use std::net::Ipv4Addr;
use std::str;
use std::time::Duration;
use url::Url;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BencodeTrackerResp {
    #[serde(default)]
    interval: u32,
    #[serde(default)]
    peers: ByteBuf,
}

impl TorrentFile {

    /* Build url get request using url encoding library */
    fn build_tracker_url(&mut self, peerid: Vec<u8>, port: u16) -> Result<String, Error> {
        let mut base = Url::parse(&self.Announce).unwrap();
        base.query_pairs_mut().append_pair("compact", "1");
        base.query_pairs_mut().append_pair("downloaded", "0");
        // Use encoding_override() to enforce binary percent encoding of info_hash
        base.query_pairs_mut()
            .encoding_override(Some(&|input| {
                if input != "!" {
                    // Return the actual value ("info_hash", in this particular case)
                    Cow::Borrowed(input.as_bytes())
                } else {
                    // When "!" is seen, return the binary data instead
                    Cow::Owned(self.InfoHash.to_owned().clone())
                }
            }))
            .append_pair("info_hash", "!");
        base.query_pairs_mut()
            .append_pair("left", &self.Length.to_string());
        base.query_pairs_mut()
            .encoding_override(Some(&|input| {
                if input != "!" {
                    // Return the actual value ("info_hash", in this particular case)
                    Cow::Borrowed(input.as_bytes())
                } else {
                    // When "!" is seen, return the binary data instead
                    Cow::Owned(peerid.clone())
                }
            }))
            .append_pair("peer_id", "!");
        base.query_pairs_mut()
            .append_pair("port", &port.to_string());
        base.query_pairs_mut().append_pair("uploaded", "0");

        Ok(base.to_string())
    }

    /* Request a list of peers from the tracker using a get request*/
    pub fn request_peers(&mut self, peerid: Vec<u8>, port: u16) -> Result<Vec<Peer>, Error> {
        let url = self.build_tracker_url(peerid, port).unwrap();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap();
        let resp = client.get(&url).send().unwrap().bytes().unwrap();
        let tracker_resp: BencodeTrackerResp = de::from_bytes::<BencodeTrackerResp>(&resp).unwrap();
        Ok(unmarshal(tracker_resp.peers.to_vec()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Matcher};

    #[test]
    fn test_build_tracker_url() {
        let mut to = TorrentFile {
            Announce: "http://bttracker.debian.org:6969/announce".to_string(),
            InfoHash: vec![
                216, 247, 57, 206, 195, 40, 149, 108, 204, 91, 191, 31, 134, 217, 253, 207, 219,
                168, 206, 182,
            ],
            PieceHashes: vec![
                vec![
                    49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 97, 98, 99, 100, 101, 102, 103, 104,
                    105, 106,
                ],
                vec![
                    97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 49, 50, 51, 52, 53, 54, 55, 56,
                    57, 48,
                ],
            ],
            PieceLength: 262144,
            Length: 351272960,
            Name: "debian-10.2.0-amd64-netinst.iso".to_string(),
        };

        let peer_id: Vec<u8> = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let port = 6881;
        let url = match to.build_tracker_url(peer_id, port) {
            Ok(url) => url,
            Err(_) => panic!(),
        };
        let expected = "http://bttracker.debian.org:6969/announce?compact=1&downloaded=0&info_hash=%D8%F79%CE%C3%28%95l%CC%5B%BF%1F%86%D9%FD%CF%DB%A8%CE%B6&left=351272960&peer_id=%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%10%11%12%13%14&port=6881&uploaded=0".to_string();
        assert_eq!(url, expected)
    }

    #[test]
    fn test_request_peer() {
        let response_struct = BencodeTrackerResp {
            interval: 900,
            peers: ByteBuf::from(vec![192, 0, 2, 123, 0x1A, 0xE1, 127, 0, 0, 1, 0x1A, 0xE9]),
        };
        let response_bencode = ser::to_bytes::<BencodeTrackerResp>(&response_struct).unwrap();
        #[allow(unused_variables)]
        let mock = mock("GET", Matcher::Any)
            .with_body(response_bencode)
            .create();

        let mut to = TorrentFile {
            Announce: mockito::server_url(),
            InfoHash: vec![
                216, 247, 57, 206, 195, 40, 149, 108, 204, 91, 191, 31, 134, 217, 253, 207, 219,
                168, 206, 182,
            ],
            PieceHashes: vec![
                vec![
                    49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 97, 98, 99, 100, 101, 102, 103, 104,
                    105, 106,
                ],
                vec![
                    97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 49, 50, 51, 52, 53, 54, 55, 56,
                    57, 48,
                ],
            ],
            PieceLength: 262144,
            Length: 351272960,
            Name: "debian-10.2.0-amd64-netinst.iso".to_string(),
        };

        let resp = to
            .request_peers(
                vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                ],
                6882,
            )
            .unwrap();
        let expected = vec![
            Peer {
                ip: Ipv4Addr::new(192, 0, 2, 123),
                port: 6881,
            },
            Peer {
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 6889,
            },
        ];
        assert_eq!(format!("{:?}", resp), format!("{:?}", expected))
        // assert_eq!(1, 0)
    }
}
