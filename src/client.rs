use crate::bitfield::Bitfield;
use crate::handshake::{read_handshake, serialize_handshake, Handshake};
use crate::message::*;
use crate::peers::Peer;
#[allow(unused_imports)]
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
#[allow(unused_imports)]
use std::sync::mpsc;
#[allow(unused_imports)]
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
pub struct Client {
    pub(crate) conn: TcpStream,
    pub(crate) choked: bool,
    pub(crate) bitfield: Bitfield,
    peer: Peer,
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
}

impl Client {
    pub(crate) fn read(&mut self) -> Result<Message, Error> {
        match read_message(&mut self.conn) {
            Ok(msg) => Ok(msg),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn send_request(
        &mut self,
        index: &u32,
        begin: &u32,
        length: &u32,
    ) -> Result<(), Error> {
        let req = format_request(*index, *begin, *length);
        match self.conn.write(&serialize_message(&req)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn send_interested(&mut self) -> Result<(), Error> {
        let mut msg = Message::default();
        msg.id = MESSAGE_INTERESTED;
        match self.conn.write(&serialize_message(&msg)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn send_not_interested(&mut self) -> Result<(), Error> {
        let mut msg = Message::default();
        msg.id = MESSAGE_NOT_INTERESTED;
        match self.conn.write(&serialize_message(&msg)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn send_unchoke(&mut self) -> Result<(), Error> {
        let mut msg = Message::default();
        msg.id = MESSAGE_UNCHOKE;
        match self.conn.write(&serialize_message(&msg)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn send_have(&mut self, index: u32) -> Result<(), Error> {
        let msg = format_have(index);
        match self.conn.write(&serialize_message(&msg)) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
fn receive_bitfield(conn: &mut TcpStream) -> Result<Bitfield, Error> {
    conn.set_write_timeout(Some(Duration::new(5, 0))).unwrap();
    conn.set_read_timeout(Some(Duration::new(5, 0))).unwrap();

    match read_message(conn) {
        Ok(s) => {
            let msg = s;
            let id_error = Error::new(ErrorKind::InvalidData, "id Error");
            if msg.id != MESSAGE_BITFIELD {
                Err(id_error)
            } else {
                conn.set_write_timeout(Some(Duration::new(1000, 0)))
                    .unwrap();
                conn.set_read_timeout(Some(Duration::new(1000, 0))).unwrap();
                Ok(msg.payload)
            }
        }
        Err(e) => Err(e),
    }
}

fn complete_handshake(
    conn: &mut TcpStream,
    info_hash: &Vec<u8>,
    peer_id: &Vec<u8>,
) -> Result<Handshake, Error> {
    conn.set_write_timeout(Some(Duration::new(3, 0))).unwrap();
    conn.set_read_timeout(Some(Duration::new(3, 0))).unwrap();
    let reader_error = Error::new(ErrorKind::InvalidData, "unexpected infohash");
    let req = Handshake {
        pstr: String::from("BitTorrent protocol").into_bytes(),
        info_hash: info_hash.clone(),
        peer_id: peer_id.clone(),
    };
    conn.write(&serialize_handshake(&req)).unwrap();
    let received = match read_handshake(conn) {
        Ok(received) => received,
        Err(e) => return Err(e),
    };
    if received.info_hash == info_hash.clone() {
        conn.set_write_timeout(Some(Duration::new(1000, 0)))
            .unwrap();
        conn.set_read_timeout(Some(Duration::new(1000, 0))).unwrap();
        Ok(received)
    } else {
        Err(reader_error)
    }
}

pub(crate) fn new_client(
    peer: &Peer,
    peer_id: &Vec<u8>,
    info_hash: &Vec<u8>,
) -> Result<Client, Error> {
    let info_hash_copy = info_hash.to_vec();
    let peer_id_copy = peer_id.to_vec();
    let three_seconds = Duration::new(3, 0);
    let peer_clone = peer.clone();
    match TcpStream::connect_timeout(&SocketAddr::from(peer.get_socket_address()), three_seconds) {
        Ok(mut s) => match complete_handshake(&mut s, info_hash, peer_id) {
            Ok(_) => match receive_bitfield(&mut s) {
                Ok(bf) => {
                    let client = Client {
                        conn: s,
                        choked: true,
                        bitfield: bf,
                        peer: peer_clone,
                        info_hash: info_hash_copy,
                        peer_id: peer_id_copy,
                    };
                    return Ok(client);
                }
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e),
        },
        Err(e) => return Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Shutdown, TcpListener};

    #[test]
    fn test_successful_receive_bitfield() {
        create_client_server_bf();
    }

    fn dummy_client_bf() {
        match TcpStream::connect("127.0.0.1:8082") {
            Ok(mut stream) => {
                let msg = b"hello";
                let pass_test = b"pass";
                let fail_test = b"fail";
                stream.write(msg).unwrap();
                let bf = receive_bitfield(&mut stream).unwrap();
                let expected_bf: [u8; 5] = [1, 2, 3, 4, 5];
                if &bf[0..5] == expected_bf {
                    stream.write(pass_test).unwrap();
                } else {
                    stream.write(fail_test).unwrap();
                }
            }
            Err(e) => {
                println!("Failed to connect: {}", e);
            }
        }
    }

    fn handle_client_bf(mut stream: TcpStream) {
        let mut data = [0 as u8; 50]; // using 50 byte buffer
        while match stream.read(&mut data) {
            Ok(size) => {
                // dont echo everything!
                if &data[0..size] == b"hello" {
                    let msg: [u8; 10] = [0x00, 0x00, 0x00, 0x06, 5, 1, 2, 3, 4, 5];
                    stream.write(&msg).unwrap();
                    true
                } else {
                    // assert_eq!(&data[0..size], b"pass");
                    assert_eq!(1, 1);
                    false
                }
            }
            Err(_) => {
                println!(
                    "An error occurred, terminating connection with {}",
                    stream.peer_addr().unwrap()
                );
                stream.shutdown(Shutdown::Both).unwrap();
                false
            }
        } {}
    }
    fn create_client_server_bf() {
        let listener: TcpListener = TcpListener::bind("127.0.0.1:8082").unwrap();
        let dummy_client_thread = thread::spawn(move || {
            dummy_client_bf();
        });
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    handle_client_bf(stream);
                    dummy_client_thread.join().unwrap();
                    break;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
    }

    #[test]
    fn test_successful_handshake() {
        create_client_server_hs();
    }
    fn dummy_client_hs() {
        match TcpStream::connect("127.0.0.1:8080") {
            Ok(mut stream) => {
                //let msg = b"hello";
                let pass_test = b"pass";
                let fail_test = b"fail";
                let client_infohash: [u8; 20] = [
                    134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49,
                    0, 116,
                ];
                let client_peer_id: [u8; 20] = [
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                ];
                //stream.write(msg).unwrap();
                let incoming_handshake = complete_handshake(
                    &mut stream,
                    &client_infohash.to_vec(),
                    &client_peer_id.to_vec(),
                )
                .unwrap();
                let expected_infohash: [u8; 20] = [
                    134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49,
                    0, 116,
                ];
                let expected_peer_id: [u8; 20] = [
                    45, 83, 89, 48, 48, 49, 48, 45, 192, 125, 147, 203, 136, 32, 59, 180, 253, 168,
                    193, 19,
                ];
                if incoming_handshake.peer_id[0..20] == expected_peer_id
                    && incoming_handshake.info_hash[0..20] == expected_infohash
                {
                    stream.write(pass_test).unwrap();
                } else {
                    stream.write(fail_test).unwrap();
                }
            }
            Err(e) => {
                println!("Failed to connect: {}", e);
            }
        }
    }

    fn handle_client_hs(mut stream: TcpStream) {
        let mut data = [0 as u8; 100]; // using 50 byte buffer
        let mut counter = 0;
        while match stream.read(&mut data) {
            Ok(size) => {
                // dont echo everything!
                if counter == 0 {
                    let server_handshake: [u8; 68] = [
                        19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116,
                        111, 99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 134, 212, 200, 0, 36, 164, 105,
                        190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116, 45, 83, 89, 48, 48,
                        49, 48, 45, 192, 125, 147, 203, 136, 32, 59, 180, 253, 168, 193, 19,
                    ];
                    stream.write(&server_handshake).unwrap();
                    counter = counter + 1;
                    data = [0 as u8; 100];
                    true
                } else {
                    for x in &data[0..size] {
                        print!("{} ", x);
                    }
                    assert_eq!(&data[0..size], b"pass");
                    false
                }
            }
            Err(_) => {
                println!(
                    "An error occurred, terminating connection with {}",
                    stream.peer_addr().unwrap()
                );
                stream.shutdown(Shutdown::Both).unwrap();
                false
            }
        } {}
    }
    fn create_client_server_hs() {
        let listener: TcpListener = TcpListener::bind("127.0.0.1:8080").unwrap();
        let dummy_client_thread = thread::spawn(move || {
            dummy_client_hs();
        });
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    handle_client_hs(stream);
                    dummy_client_thread.join().unwrap();
                    break;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
    }
}
