#![allow(non_snake_case)]
extern crate bencodex;
extern crate crypto;
extern crate hex;
extern crate serde_bencode;
extern crate serde_bytes;
use crate::p2p::*;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use serde_bencode::{de, ser};
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};

static DEFAULT_PORT: u16 = 6881;

#[derive(Debug, Deserialize, Serialize)]
pub struct TorrentFile {
    pub(crate) Announce: String,
    pub(crate) InfoHash: Vec<u8>,
    pub(crate) PieceHashes: Vec<Vec<u8>>,
    pub(crate) PieceLength: u32,
    pub(crate) Length: u32,
    pub(crate) Name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BencodeInfo {
    #[serde(default)]
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piecelength: u32,
    #[serde(default)]
    length: u32,
    #[serde(default)]
    name: String,
}

/* Struct for recieving results of a bencode deserialize */
#[derive(Debug, Deserialize, Serialize)]
pub struct BencodeTorrent {
    #[serde(default)]
    pub announce: String,
    info: BencodeInfo,
}

/* Deserialize bencoded file into BencodeTorrent object. */
pub fn open(path: String) -> Result<TorrentFile, Error> {
    let mut outfile = match File::open(path) {
        Ok(outfile) => outfile,
        Err(err) => return Err(err),
    };
    let mut buffer: Vec<u8> = Vec::new();
    outfile.read_to_end(&mut buffer).unwrap();
    let mut bto = de::from_bytes::<BencodeTorrent>(&buffer).unwrap();
    return bto.to_torrent_file();
}

impl BencodeTorrent {

    /* Convert BencodeTorrent to more useable struct TorrentFile */
    pub fn to_torrent_file(&mut self) -> Result<TorrentFile, Error> {
        let info_hash = self.info.hash();
        let piece_hashes = match self.info.split_piece_hashes() {
            Ok(piece_hashes) => piece_hashes,
            Err(err) => return Err(err),
        };
        let t = TorrentFile {
            Announce: self.announce.to_owned(),
            InfoHash: info_hash,
            PieceHashes: piece_hashes,
            PieceLength: self.info.piecelength,
            Length: self.info.length,
            Name: self.info.name.to_owned(),
        };
        return Ok(t);
    }
}
impl BencodeInfo {
    pub fn hash(&mut self) -> Vec<u8> {
        let buffer: Vec<u8> = ser::to_bytes::<BencodeInfo>(self).unwrap();
        let mut h = Sha1::new();
        h.input(&buffer);
        let result: Vec<u8> = hex::decode(h.result_str()).unwrap();
        result
    }

    /* Convert piece hashes from bytebuffer to Vec of Vec for ergonomics */
    pub fn split_piece_hashes(&mut self) -> Result<Vec<Vec<u8>>, Error> {
        let hash_length = 20; //length of sha1 hash
        let buffer = self.pieces.to_owned();
        if buffer.len() % hash_length != 0 {
            let err = Error::new(ErrorKind::Other, "oh no!");
            return Err(err);
        }
        let num_hashes = buffer.len() / hash_length;
        let mut hashes: Vec<Vec<u8>> = vec![vec![0; 20]; num_hashes];
        for i in 0..num_hashes {
            hashes[i] = buffer[i * hash_length..(i + 1) * hash_length].to_vec();
        }
        Ok(hashes)
    }
}
impl TorrentFile {

    /*  */
    pub fn download_to_file(&mut self, path: String) -> Result<(), Error> {
        let mut peerid: Vec<u8> = vec![0; 20];

        for x in peerid.iter_mut() {
            *x = rand::random()
        }

        let peers = match self.request_peers(peerid.to_vec(), DEFAULT_PORT) {
            Ok(peers) => peers,
            Err(err) => return Err(err),
        };

        let mut torrent = Torrent {
            peers,
            peer_id: peerid.to_vec(),
            info_hash: self.InfoHash.to_vec(),
            piece_hashes: self.PieceHashes.to_vec(),
            piece_length: self.PieceLength,
            length: self.Length,
            name: self.Name.to_string(),
        };

        let buf = match torrent.download() {
            Ok(buf) => buf,
            Err(err) => return Err(err),
        };

        let mut outfile = match File::create(path) {
            Ok(outfile) => outfile,
            Err(err) => return Err(err),
        };

        match outfile.write(&buf) {
            Ok(_) => return Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    // fn test_open() {
    //     let torrent = open(String::from("C:/Users/Bernardo/git-repo/rust-torrent/src/testdata/archlinux-2019.12.01-x86_64.iso.torrent")).unwrap();
    //     let mut file = File::open(String::from("C:/Users/Bernardo/git-repo/rust-torrent/src/testdata/archlinux-2019.12.01-x86_64.iso.torrent.golden.json")).unwrap();
    //     let mut golden = String::new();
    //     file.read_to_string(&mut golden).unwrap();
    //     let expected: TorrentFile = serde_json::from_str(&golden).unwrap();
    //     assert_eq!(format!("{:?}", expected), format!("{:?}", torrent));
    // }

    #[test]
    fn test_to_torrent_file_correct_conversion() {
        let mut input = BencodeTorrent {
            announce: "http://bttracker.debian.org:6969/announce".to_string(),
            info: BencodeInfo {
                pieces: ByteBuf::from("1234567890abcdefghijabcdefghij1234567890"),
                piecelength: 262144,
                length: 351272960,
                name: "debian-10.2.0-amd64-netinst.iso".to_string(),
            },
        };

        let output = TorrentFile {
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

        let result = input.to_torrent_file().unwrap();
        assert_eq!(format!("{:?}", result), format!("{:?}", output));
    }

    #[test]
    fn test_to_torrent_file_insufficient_bytes() {
        let mut input = BencodeTorrent {
            announce: "http://bttracker.debian.org:6969/announce".to_string(),
            info: BencodeInfo {
                pieces: ByteBuf::from("1234567890abcdefghijabcdef"),
                piecelength: 262144,
                length: 351272960,
                name: "debian-10.2.0-amd64-netinst.iso".to_string(),
            },
        };
        match input.to_torrent_file() {
            Ok(_) => assert_eq!(1, 0),
            Err(_) => assert_eq!(1, 1),
        }
    }
}
