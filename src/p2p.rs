#![allow(non_snake_case)]
extern crate crypto;
use crate::bitfield::*;
use crate::client::*;
use crate::message::*;
use crate::peers::*;
use crossbeam_channel::unbounded;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::io::{Error, ErrorKind};
use std::thread;
use std::time::Duration;

static MAX_BLOCK_SIZE: u32 = 16384;
static MAX_BACK_LOG: u32 = 5;

#[derive(Clone)]
pub struct Torrent {
    pub(crate) peers: Vec<Peer>,
    pub(crate) peer_id: Vec<u8>,
    pub(crate) info_hash: Vec<u8>,
    pub(crate) piece_hashes: Vec<Vec<u8>>,
    pub(crate) piece_length: u32,
    pub(crate) length: u32,
    pub(crate) name: String,
}

pub struct PieceWork {
    pub(crate) index: u32,
    pub(crate) hash: Vec<u8>,
    pub(crate) length: u32,
}

pub struct PieceResult {
    pub(crate) index: u32,
    pub(crate) buf: Vec<u8>,
}

pub struct PieceProgress {
    pub(crate) index: u32,
    pub(crate) buf: Vec<u8>,
    pub(crate) downloaded: u32,
    pub(crate) requested: u32,
    pub(crate) backlog: u32,
}

impl PieceProgress {
    fn read_message(&mut self, c: &mut Client) -> Result<u32, Error> {
        match c.read() {
            Ok(msg) => match msg.id {
                1 => {
                    c.choked = false;
                    Ok(1)
                }
                0 => {
                    c.choked = true;
                    Ok(1)
                }
                4 => match parse_have(&msg) {
                    Ok(index) => {
                        c.bitfield = set_piece(&c.bitfield, index as usize);
                        drop(c);
                        Ok(1)
                    }
                    Err(e) => Err(e),
                },
                7 => match parse_piece(self.index, &mut self.buf, &msg) {
                    Ok(n) => {
                        self.downloaded += n;
                        self.backlog -= 1;
                        drop(c);
                        Ok(1)
                    }
                    Err(e) => Err(e),
                },
                _ => Ok(2),
            },
            Err(e) => Err(e),
        }
    }
}

fn attempt_download_piece(c: &mut Client, pw: &PieceWork) -> Result<Vec<u8>, Error> {
    let mut state = PieceProgress {
        index: pw.index,
        buf: vec![0; pw.length as usize],
        downloaded: 0,
        requested: 0,
        backlog: 0,
    };
    c.conn
        .set_write_timeout(Some(Duration::new(30, 0)))
        .unwrap();
    c.conn.set_read_timeout(Some(Duration::new(30, 0))).unwrap();
    while state.downloaded < pw.length {
        if !c.choked.clone() {
            while state.backlog < MAX_BACK_LOG && state.requested < pw.length {
                let mut block_size = MAX_BLOCK_SIZE;
                if (pw.length - state.requested) < block_size {
                    block_size = pw.length - state.requested;
                }
                match c.send_request(&pw.index, &state.requested, &block_size) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
                state.backlog += 1;
                state.requested += block_size;
            }
        }
        match state.read_message(c) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }
    c.conn
        .set_write_timeout(Some(Duration::new(1000, 0)))
        .unwrap();
    c.conn
        .set_read_timeout(Some(Duration::new(1000, 0)))
        .unwrap();
    Ok(state.buf.clone())
}

fn check_integrity(pw: &PieceWork, buf: Vec<u8>) -> Result<(), Error> {
    let reader_error = Error::new(ErrorKind::InvalidData, "unexpected infohash");
    let mut h = Sha1::new();
    h.input(&buf);
    let hash_output = hex::decode(h.result_str()).unwrap();
    if hash_output != pw.hash {
        Err(reader_error)
    } else {
        Ok(())
    }
}

impl Torrent {

    
    fn start_download_work(
        &mut self,
        peer: Peer,
        workQueue: (
            crossbeam_channel::Sender<PieceWork>,
            crossbeam_channel::Receiver<PieceWork>,
        ),
        results: crossbeam_channel::Sender<PieceResult>,
    ) {
        let mut c = match new_client(&peer, &self.peer_id, &self.info_hash) {
            Ok(c) => c,
            Err(_) => {
                println!("Could not handshake, disconnecting");
                return;
            }
        };

        c.send_unchoke().unwrap();
        c.send_interested().unwrap();
        println!("Completed handshake with {}\n", peer.ip);

        loop {
            let pw = match workQueue.1.recv() {
                Ok(pw) => pw,
                Err(_) => return,
            };

            let bf = &mut c.bitfield.clone();

            if !has_piece(bf, pw.index as usize) {
                workQueue.0.send(pw).unwrap();
                continue;
            }

            let buf = match attempt_download_piece(&mut c, &pw) {
                Ok(buf) => buf,
                Err(_) => {
                    println!("Exiting");
                    workQueue.0.send(pw).unwrap();
                    return;
                }
            };

            let pindex1 = pw.index.clone();
            let pindex2 = pw.index.clone();
            let cbuf = buf.clone();
            match check_integrity(&pw, buf) {
                Err(_) => {
                    println!("Piece {} failed integrity check", pindex1);
                    workQueue.0.send(pw).unwrap();
                    continue;
                }
                Ok(_) => {
                    c.send_have(pindex1).unwrap();
                    results
                        .send(PieceResult {
                            index: pindex2,
                            buf: cbuf,
                        })
                        .unwrap();
                }
            }
        }
    }

    fn calculate_bounds_for_piece(&self, index: u32) -> (u32, u32) {
        let begin = index * self.piece_length;
        let mut end = begin + self.piece_length;
        if end > self.length {
            end = self.length;
        }
        (begin, end)
    }

    fn calculate_piece_size(&self, index: u32) -> u32 {
        let (begin, end) = self.calculate_bounds_for_piece(index);
        end - begin
    }

    /* initialize channels, fill work queue with work, create thread for each peer , put together data as work is done */
    pub fn download(&mut self) -> Result<Vec<u8>, Error> {
        println!("Starting download for {}", self.name);
        let workQueue: (
            crossbeam_channel::Sender<PieceWork>,
            crossbeam_channel::Receiver<PieceWork>,
        ) = unbounded();
        let results: (
            crossbeam_channel::Sender<PieceResult>,
            crossbeam_channel::Receiver<PieceResult>,
        ) = unbounded();
        for index in 0..self.piece_hashes.len() {
            let length = self.calculate_piece_size(index as u32);
            let hash = self.piece_hashes[index].clone();
            let work = PieceWork {
                index: (index as u32),
                hash,
                length,
            };
            workQueue.0.send(work).unwrap();
        }
        let peers_in_box = self.peers.to_owned();
        for peer in peers_in_box {
            let mut self_copy = self.clone();
            let workQueueCopy = (workQueue.0.clone(), workQueue.1.clone());
            let resultsCopy = results.0.clone();
            thread::spawn(move || {
                self_copy.start_download_work(peer.clone(), workQueueCopy, resultsCopy);
            });
        }
        let mut buffer: Vec<u8> = vec![0; self.length as usize];
        let mut done_pieces = 0;
        let num_of_hashes = self.piece_hashes.len();
        while done_pieces < num_of_hashes {
            let res = results.1.recv().unwrap();
            let (begin, _) = self.calculate_bounds_for_piece(res.index);
            for k in 0..res.buf.len() {
                buffer[begin as usize + k] = res.buf[k];
            }
            done_pieces = done_pieces + 1;
            let percent = ((done_pieces as f64) / (self.piece_hashes.len() as f64)) * (100 as f64);
            let num_of_workers = self.peers.len();
            println!(
                "{:.2}% downloaded piece {} from {} peers\n",
                percent, res.index, num_of_workers
            );
        }
        Ok(buffer)
    }
}
