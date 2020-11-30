use byteorder::{BigEndian, ReadBytesExt};
#[allow(unused_imports)]
use std::fmt;
#[allow(unused_imports)]
use std::io::{Cursor, Error, ErrorKind, Read, Result};
use std::net::{Ipv4Addr, SocketAddrV4};
#[allow(unused_imports)]
use std::str;
use std::vec::Vec;

#[derive(Copy, Clone, Debug)]
pub struct Peer {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl Peer {
    pub(crate) fn get_socket_address(&self) -> SocketAddrV4 {
        return SocketAddrV4::new(self.ip, self.port);
    }
}

/* convert byte vec representation of peers into vec of peer structs */
pub(crate) fn unmarshal(peers_bin: Vec<u8>) -> Result<Vec<Peer>> {
    let peer_size = 6;
    let num_peers = peers_bin.len() / peer_size;
    let peer_error = Error::new(ErrorKind::InvalidData, "Malformed Peers");
    if peers_bin.len() % peer_size != 0 {
        Err(peer_error)
    } else {
        let mut peers: Vec<Peer> = vec![
            Peer {
                ip: Ipv4Addr::new(1, 1, 1, 1),
                port: 0
            };
            num_peers
        ];
        let mut port_vec = vec![];
        for i in 0..num_peers {
            let offset = i * peer_size;
            port_vec.push(peers_bin[offset + 4]);
            port_vec.push(peers_bin[offset + 5]);
            let port_vc = &port_vec[0..2].to_vec();
            let mut port_cursor = Cursor::new(port_vc);
            peers[i].ip = Ipv4Addr::new(
                peers_bin[offset],
                peers_bin[offset + 1],
                peers_bin[offset + 2],
                peers_bin[offset + 3],
            );
            peers[i].port = port_cursor.read_u16::<BigEndian>().unwrap();
            port_vec = vec![];
        }
        Ok(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unmarshal_correctly_parse_peers() {
        let input = vec![127, 0, 0, 1, 0x00, 0x50, 1, 1, 1, 1, 0x01, 0xbb];
        let peer_one = Peer {
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 80,
        };
        let peer_two = Peer {
            ip: Ipv4Addr::new(1, 1, 1, 1),
            port: 443,
        };
        let mut output: Vec<Peer> = vec![];
        output.push(peer_one);
        output.push(peer_two);
        match unmarshal(input) {
            Ok(_s) => assert_eq!(_s[0].ip, output[0].ip),
            Err(_err) => assert_eq!(1, 2),
        }
    }

    #[test]
    fn test_unmarshal_malformed_peer() {
        let input = vec![127, 0, 0, 1, 0x00];
        match unmarshal(input) {
            Ok(_s) => assert_eq!(1, 2),
            Err(_err) => assert_eq!(1, 1),
        }
    }

    #[test]
    fn test_get_socket_address() {
        let input = Peer {
            ip: Ipv4Addr::new(127, 0, 0, 1),
            port: 8080,
        };
        let expected_output = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
        assert_eq!(input.get_socket_address(), expected_output);
    }
}
