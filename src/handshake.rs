#[allow(unused_imports)]
use bytes::{BufMut, BytesMut};
#[allow(unused_imports)]
use std::io::{Error, ErrorKind, Read, Result, Write};
#[allow(unused_imports)]
use std::net::{TcpListener, TcpStream};
#[allow(unused_imports)]
use std::thread;

pub struct Handshake {
    pub(crate) pstr: Vec<u8>,
    pub(crate) info_hash: Vec<u8>,
    pub(crate) peer_id: Vec<u8>,
}

#[allow(dead_code)]
pub fn new_handshake() -> Handshake {
    Handshake {
        pstr: String::from("BitTorrent protocol").into_bytes(),
        info_hash: vec![0, 1],
        peer_id: vec![0, 1],
    }
}

#[allow(dead_code)]
pub fn new_handshake_with_input(info_hash: Vec<u8>, peer_id: Vec<u8>) -> Handshake {
    Handshake {
        pstr: String::from("BitTorrent protocol").into_bytes(),
        info_hash,
        peer_id,
    }
}
pub fn serialize_handshake(hs: &Handshake) -> Vec<u8> {
    let mut buf: Vec<u8> = vec![];
    buf.push(hs.pstr.len() as u8);
    let mut pstr_as_vec: Vec<u8> = hs.pstr.clone();
    buf.append(&mut pstr_as_vec);
    let mut reserved_bytes = vec![0; 8];
    buf.append(&mut reserved_bytes);
    let mut info_hash_as_vec: Vec<u8> = hs.info_hash.clone();
    buf.append(&mut info_hash_as_vec);
    let mut peer_id_as_vec: Vec<u8> = hs.peer_id.clone();
    buf.append(&mut peer_id_as_vec);
    buf
}

pub fn read_handshake(reader: &mut TcpStream) -> Result<Handshake> {
    let mut length_buffer: [u8; 1] = [0; 1];
    match reader.read_exact(&mut length_buffer) {
        Err(e) => return Err(e),
        _ => (),
    }
    let pstrlen = length_buffer[0];
    let reader_error = Error::new(ErrorKind::InvalidData, "unexpected infohash");
    if pstrlen == 0 {
        return Err(reader_error);
    }
    let mut handshakebuf: Vec<u8> = vec![0; 48 + (pstrlen as usize)];
    match reader.read_exact(&mut handshakebuf) {
        Err(e) => return Err(e),
        _ => (),
    }
    let mut infohash = Vec::new();
    let mut peerid = Vec::new();
    let mut pstr = Vec::new();
    for (i, x) in handshakebuf.iter().enumerate() {
        if i < pstrlen as usize {
            pstr.push(x.to_owned());
        }
        if i >= (pstrlen as usize + 8) && i < (pstrlen as usize + 8 + 20) {
            infohash.push(x.to_owned());
        }
        if i >= (pstrlen as usize + 8 + 20) {
            peerid.push(x.to_owned());
        }
    }
    let h = Handshake {
        pstr: pstr,
        info_hash: infohash,
        peer_id: peerid,
    };
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_info_hash() {
        let input_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let input_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let h = new_handshake_with_input(input_info_hash.to_vec(), input_peer_id.to_vec());
        let expected = Handshake {
            pstr: String::from("BitTorrent protocol").into_bytes(),
            info_hash: input_info_hash.to_vec(),
            peer_id: input_peer_id.to_vec(),
        };
        assert_eq!(h.info_hash, expected.info_hash);
    }

    #[test]
    fn test_new_peer_id() {
        let input_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let input_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let h = new_handshake_with_input(input_info_hash.to_vec(), input_peer_id.to_vec());
        let expected = Handshake {
            pstr: String::from("BitTorrent protocol").into_bytes(),
            info_hash: input_info_hash.to_vec(),
            peer_id: input_peer_id.to_vec(),
        };
        assert_eq!(h.peer_id, expected.peer_id);
    }

    #[test]
    fn test_new_pstr() {
        let input_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let input_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let h = new_handshake_with_input(input_info_hash.to_vec(), input_peer_id.to_vec());
        let expected = Handshake {
            pstr: String::from("BitTorrent protocol").into_bytes(),
            info_hash: input_info_hash.to_vec(),
            peer_id: input_peer_id.to_vec(),
        };
        assert_eq!(h.pstr, expected.pstr);
    }

    #[test]
    fn test_serialize_one() {
        let input_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let input_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let input = new_handshake_with_input(input_info_hash.to_vec(), input_peer_id.to_vec());
        let output = vec![
            19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99,
            111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90,
            16, 44, 247, 23, 128, 49, 0, 116, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20,
        ];
        assert_eq!(serialize_handshake(&input), output);
    }

    #[test]
    fn test_serialize_two() {
        let input_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let input_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let input_pstr = String::from("BitTorrent protocol, but cooler?");
        let input = Handshake {
            pstr: input_pstr.into_bytes(),
            info_hash: input_info_hash.to_vec(),
            peer_id: input_peer_id.to_vec(),
        };
        let output = vec![
            32, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99,
            111, 108, 44, 32, 98, 117, 116, 32, 99, 111, 111, 108, 101, 114, 63, 0, 0, 0, 0, 0, 0,
            0, 0, 134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49,
            0, 116, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        assert_eq!(serialize_handshake(&input), output);
    }

    #[test]
    fn test_read_default_pstr() {
        let output_info_hash: [u8; 20] = [
            134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116,
        ];
        let output_peer_id: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let output_pstr = String::from("BitTorrent protocol");
        let output = Handshake {
            pstr: output_pstr.into_bytes(),
            info_hash: output_info_hash.to_vec(),
            peer_id: output_peer_id.to_vec(),
        };
        thread::spawn(move || {
            mock_tpc_read_handshake();
        });
        match TcpStream::connect("127.0.0.1:8081") {
            Ok(mut stream) => {
                match read_handshake(&mut stream) {
                    Ok(_s) => assert_eq!(_s.pstr, output.pstr),
                    Err(_err) => assert_eq!(1, 2),
                };
            }
            Err(e) => {
                println!("Failed to connect: {}", e);
            }
        }
    }

    fn mock_tpc_read_handshake() {
        let listener: TcpListener = TcpListener::bind("127.0.0.1:8081").unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    let input: [u8; 68] = [
                        19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116,
                        111, 99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 134, 212, 200, 0, 36, 164, 105,
                        190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116, 1, 2, 3, 4, 5, 6,
                        7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                    ];
                    stream.write(&input).unwrap();
                }
                Err(e) => {
                    println!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
    }
}
//
//    # [test]
//    fn test_read_default_peer_id(){
//        let input: [u8; 68] = [19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
//        let mut output_info_hash: [u8; 20] = [134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116];
//        let mut output_peer_id: [u8; 20] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
//        let mut output_pstr = String::from("BitTorrent protocol");
//        let output = Handshake{
//            pstr: output_pstr,
//            info_hash: output_info_hash.to_vec(),
//            peer_id: output_peer_id.to_vec(),
//        };
//        let input_as_vec : Vec<u8> = input.to_vec();
//        let mut input_reader = Cursor::new(input_as_vec);
//        match read_handshake(&mut input_reader){
//            Ok(_s) => assert_eq!(_s.peer_id, output.peer_id),
//            Err(_err) => assert_eq!(1, 2)
//        };
//    }
//
//    # [test]
//    fn test_read_default_info_hash(){
//        let input: [u8; 68] = [19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
//        let mut output_info_hash: [u8; 20] = [134, 212, 200, 0, 36, 164, 105, 190, 76, 80, 188, 90, 16, 44, 247, 23, 128, 49, 0, 116];
//        let mut output_peer_id: [u8; 20] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
//        let mut output_pstr = String::from("BitTorrent protocol");
//        let output = Handshake{
//            pstr: output_pstr,
//            info_hash: output_info_hash.to_vec(),
//            peer_id: output_peer_id.to_vec(),
//        };
//        let input_as_vec : Vec<u8> = input.to_vec();
//        let mut input_reader = Cursor::new(input_as_vec);
//        match read_handshake(&mut input_reader){
//            Ok(_s) => assert_eq!(_s.info_hash, output.info_hash),
//            Err(_err) => assert_eq!(1, 2)
//        };
//    }

//    # [test]
//    fn test_read_empty(){
//        let input_as_vec = vec![];
//        let mut input_reader = Cursor::new(input_as_vec);
//        match read_handshake(&mut input_reader){
//            Ok(_s) => assert_eq!(1, 2),
//            Err(_err) => assert_eq!(1, 1)
//        };
//    }

//    # [test]
//    fn test_read_too_few_bytes(){
//        let input_as_vec = vec![19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99, 111];
//        let mut input_reader = Cursor::new(input_as_vec);
//        match read_handshake(&mut input_reader){
//            Ok(_s) => assert_eq!(1, 2),
//            Err(_err) => assert_eq!(1, 1)
//        };
//    }
//}
