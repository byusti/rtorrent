use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
#[allow(unused_imports)]
use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};
#[allow(unused_imports)]
use std::net::{TcpListener, TcpStream};
#[allow(unused_imports)]
use std::thread;
use std::vec::Vec;

type MessageID = u8;

#[allow(dead_code)]
pub static MESSAGE_CHOKE: MessageID = 0;
pub static MESSAGE_UNCHOKE: MessageID = 1;
pub static MESSAGE_INTERESTED: MessageID = 2;
pub static MESSAGE_NOT_INTERESTED: MessageID = 3;
pub static MESSAGE_HAVE: MessageID = 4;
pub static MESSAGE_BITFIELD: MessageID = 5;
pub static MESSAGE_REQUEST: MessageID = 6;
pub static MESSAGE_PIECE: MessageID = 7;
#[allow(dead_code)]
pub static MESSAGE_CANCEL: MessageID = 8;
pub static MESSAGE_EMPTY: MessageID = 9;

pub struct Message {
    pub(crate) id: MessageID,
    pub(crate) payload: Vec<u8>,
}

impl Default for Message {
    fn default() -> Message {
        Message {
            id: MESSAGE_EMPTY,
            payload: vec![],
        }
    }
}

pub fn format_request(index: u32, begin: u32, length: u32) -> Message {
    let mut payload: Vec<u8> = vec![];
    payload.write_u32::<BigEndian>(index).unwrap();
    payload.write_u32::<BigEndian>(begin).unwrap();
    payload.write_u32::<BigEndian>(length).unwrap();
    let msg = Message {
        id: MESSAGE_REQUEST,
        payload: payload,
    };
    msg
}

pub fn format_have(index: u32) -> Message {
    let mut payload: Vec<u8> = vec![];
    payload.write_u32::<BigEndian>(index).unwrap();
    let msg = Message {
        id: MESSAGE_HAVE,
        payload: payload,
    };
    msg
}

pub fn parse_piece(index: u32, buf: &mut Vec<u8>, msg: &Message) -> Result<u32> {
    //Err(Error::new(ErrorKind::InvalidData, "Unexpected ID"))

    if msg.id != MESSAGE_PIECE {
        return Err(Error::new(ErrorKind::InvalidData, "Expected Piece"));
    }

    if msg.payload.len() < 8 {
        return Err(Error::new(ErrorKind::InvalidData, "Payload too short"));
    }

    let mut payload_read = Cursor::new(&msg.payload[0..4]);
    let parsed_index = payload_read.read_u32::<BigEndian>().unwrap();

    if parsed_index != index {
        return Err(Error::new(ErrorKind::InvalidData, "Unexpected index"));
    }

    payload_read = Cursor::new(&msg.payload[4..8]);
    let begin = payload_read.read_u32::<BigEndian>().unwrap();
    if begin >= (buf.len() as u32) {
        return Err(Error::new(ErrorKind::InvalidData, "Begin Offset too high"));
    }

    let data = &msg.payload[8..].to_vec();
    if (begin + data.len() as u32) > buf.len() as u32 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "data too long for offset",
        ));
    }
    for k in 0..data.len() {
        buf[begin as usize + k] = data[k];
    }
    return Ok(data.len() as u32);
}

pub(crate) fn parse_have(msg: &Message) -> Result<u32> {
    if msg.id != MESSAGE_HAVE {
        Err(Error::new(ErrorKind::InvalidData, "unexpected ID"))
    } else if msg.payload.len() != 4 {
        Err(Error::new(
            ErrorKind::InvalidData,
            "Incorrect payload length, length must equal 4",
        ))
    } else {
        let mut payload_read = Cursor::new(&msg.payload);
        Ok(payload_read.read_u32::<BigEndian>().unwrap())
    }
}

pub(crate) fn serialize_message(msg: &Message) -> Vec<u8> {
    let compare: Vec<u8> = vec![];
    if msg.payload == compare {
        vec![0; 4]
    } else {
        let length: u32 = (msg.payload.len() + 1) as u32;
        let mut buf: Vec<u8> = vec![];
        buf.write_u32::<BigEndian>(length).unwrap();
        let mut buf_ending: Vec<u8> = vec![0; length as usize];
        buf.append(&mut buf_ending);
        buf[4] = msg.id;
        let mut counter = 5;
        for i in &msg.payload {
            buf[counter] = *i;
            counter = counter + 1;
        }
        buf
    }
}

pub fn read_message(reader: &mut TcpStream) -> Result<Message> {
    let mut length_buffer = vec![0; 4];
    let mut reader_error = Error::new(ErrorKind::InvalidData, "unexpected ID");
    let mut reader_error_change = false;
    //implement read_exact here dummy
    match reader.read_exact(&mut length_buffer) {
        Ok(_s) => ((), ()),
        Err(_err) => return Err(reader_error), //LOOK HERE
    };
    if reader_error_change == true {
        Err(reader_error)
    } else {
        let mut length_buffer_to_cursor = Cursor::new(length_buffer);
        let length = length_buffer_to_cursor.read_u32::<BigEndian>().unwrap();
        if length == 0 {
            Ok(Message::default())
        } else {
            let mut message_buffer = vec![0; length as usize];
            match reader.read_exact(&mut message_buffer) {
                Ok(_s) => ((), ()),
                Err(_err) => (reader_error = _err, reader_error_change = true),
            };
            if reader_error_change == true {
                Err(reader_error)
            } else {
                let output_id = message_buffer[0];
                let output_payload: Vec<u8> = message_buffer[1..length as usize].to_vec();
                let output_message = Message {
                    id: output_id,
                    payload: output_payload,
                };
                Ok(output_message)
            }
        }
    }
}

#[allow(dead_code)]
fn name(msg: &Message) -> String {
    if msg.id == 9 {
        String::from("KeepAlive")
    } else {
        match msg.id {
            0 => String::from("Choke"),
            1 => String::from("Unchoke"),
            2 => String::from("Interested"),
            3 => String::from("NotInterested"),
            4 => String::from("Have"),
            5 => String::from("Bitfield"),
            6 => String::from("Request"),
            7 => String::from("Piece"),
            8 => String::from("Cancel"),
            _ => String::from(format!("Unknown#{}", msg.id)),
        }
    }
}

#[allow(dead_code)]
fn string(msg: &Message) -> String {
    if msg.id == 9 {
        name(msg)
    } else {
        format!("{} [{}]", name(msg), msg.payload.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_request() {
        let msg = format_request(4, 567, 4321);
        let expected_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x02, 0x37, 0x00, 0x00, 0x10, 0xe1,
        ];
        let expected = Message {
            id: MESSAGE_REQUEST,
            payload: expected_payload,
        };
        assert_eq!(expected.id, msg.id);
        //assert_eq!(expected.Payload, msg.Payload);
    }

    #[test]
    fn test_format_have() {
        let msg = format_have(4);
        let expected_payload: Vec<u8> = vec![0x00, 0x00, 0x00, 0x04];
        let expected = Message {
            id: MESSAGE_HAVE,
            payload: expected_payload,
        };
        assert_eq!(expected.id, msg.id);
        //assert_eq!(expected.Payload, msg.Payload);
    }

    #[test]
    fn test_parse_piece_normal() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        let output_buffer = vec![0x00, 0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x00];
        #[allow(unused_variables)]
        let length_test = parse_piece(input_index, &mut input_buffer, input_message);
        assert_eq!(input_buffer, output_buffer);
        //assert_eq!(output_length, length_test);
    }

    #[test]
    fn test_parse_piece_wrong_type() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        match parse_piece(input_index, &mut input_buffer, input_message) {
            Ok(_) => assert_eq!(1, 2),
            Err(_) => assert_eq!(1, 1),
        }
    }

    #[test]
    fn test_parse_piece_too_short() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x04, // Index
            0x00, 0x00, 0x00,
        ];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        match parse_piece(input_index, &mut input_buffer, input_message) {
            Ok(_) => assert_eq!(1, 2),
            Err(_) => {
                // println!("{}", e);
                // assert_eq!(1, 2)
                assert_eq!(1, 1)
            }
        }
    }

    #[test]
    fn test_parse_piece_wrong_index() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        match parse_piece(input_index, &mut input_buffer, input_message) {
            Ok(_) => assert_eq!(1, 2),
            Err(e) => {
                assert_eq!("Unexpected index", e.to_string())
                // assert_eq!(1, 1)
            }
        }
    }

    #[test]
    fn test_parse_piece_offset_too_high() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x04, // Index is 6, not 4
            0x00, 0x00, 0x00, 0x0c, // Begin is 12 > 10
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        match parse_piece(input_index, &mut input_buffer, input_message) {
            Ok(_) => assert_eq!(1, 2),
            Err(e) => {
                assert_eq!("Begin Offset too high", e.to_string())
                // assert_eq!(1, 1)
            }
        }
    }

    #[test]
    fn test_parse_piece_payload_too_long() {
        let input_index = 4;
        let mut input_buffer = vec![0; 10];
        let input_payload: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x04, // Index is 6, not 4
            0x00, 0x00, 0x00, 0x02, // Begin is ok
            // Block is 10 long but begin=2; too long for input buffer
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x0a, 0x0b, 0x0c, 0x0d,
        ];
        let input_message = &Message {
            id: MESSAGE_PIECE,
            payload: input_payload,
        };
        match parse_piece(input_index, &mut input_buffer, input_message) {
            Ok(_) => assert_eq!(1, 2),
            Err(e) => {
                assert_eq!("data too long for offset", e.to_string())
                // assert_eq!(1, 1)
            }
        }
    }

    #[test]
    fn test_parse_have() {
        let input_payload: Vec<u8> = vec![0x00, 0x00, 0x00, 0x04];
        let input_message = &Message {
            id: MESSAGE_HAVE,
            payload: input_payload,
        };
        let output: u32 = 4;
        match parse_have(input_message) {
            Ok(_s) => assert_eq!(_s, output),
            Err(_err) => assert_eq!(1, 2),
        }
    }

    #[test]
    fn test_serialize_message() {
        let input_payload: Vec<u8> = vec![1, 2, 3, 4];
        let input_message = Message {
            id: MESSAGE_HAVE,
            payload: input_payload,
        };
        let output: Vec<u8> = vec![0, 0, 0, 5, 4, 1, 2, 3, 4];
        assert_eq!(serialize_message(&input_message), output);
    }

    #[test]
    fn test_serialize_empty() {
        let input_payload: Vec<u8> = vec![];
        let input_message = Message {
            id: 0,
            payload: input_payload,
        };
        let output: Vec<u8> = vec![0, 0, 0, 0];
        assert_eq!(serialize_message(&input_message), output);
    }

    // #[test]
    // fn test_read_message_too_short() {
    //     thread::spawn(move || {
    //         mock_tpc_read_message_too_short();
    //     });

    //     let mut reader = TcpStream::connect("127.0.0.1:8070").unwrap();
    //     match read_message(&mut reader) {
    //         Ok(_s) => assert_eq!(1, 2),
    //         Err(_err) => assert_eq!(1, 1),
    //     }
    // }

    // fn mock_tpc_read_message_too_short() {
    //     let listener: TcpListener = TcpListener::bind("127.0.0.1:8070").unwrap();
    //     for stream in listener.incoming() {
    //         match stream {
    //             Ok(mut stream) => {
    //                 println!("New connection: {}", stream.peer_addr().unwrap());
    //                 let input: [u8; 3] = [1, 2, 3];
    //                 stream.write(&input).unwrap();
    //             }
    //             Err(e) => {
    //                 println!("Error: {}", e);
    //                 /* connection failed */
    //             }
    //         }
    //     }
    // }

    // #[test]
    // fn test_read_message_length_too_short_for_length() {
    //     thread::spawn(move || {
    //         mock_tpc_read_message_too_short_for_length();
    //     });

    //     let mut reader = TcpStream::connect("127.0.0.1:8071").unwrap();

    //     match read_message(&mut reader) {
    //         Ok(_s) => assert_eq!(1, 2),
    //         Err(_err) => assert_eq!(1, 1),
    //     }
    // }

    // fn mock_tpc_read_message_too_short_for_length() {
    //     let listener: TcpListener = TcpListener::bind("127.0.0.1:8071").unwrap();
    //     for stream in listener.incoming() {
    //         match stream {
    //             Ok(mut stream) => {
    //                 println!("New connection: {}", stream.peer_addr().unwrap());
    //                 let input: [u8; 7] = [0, 0, 0, 5, 4, 1, 2];
    //                 stream.write(&input).unwrap();
    //             }
    //             Err(e) => {
    //                 println!("Error: {}", e);
    //                 /* connection failed */
    //             }
    //         }
    //     }
    // }

    // #[test]
    // fn test_read_message_keep_alive() {
    //     let output = Message::default();
    //     thread::spawn(move || {
    //         mock_tpc_read_message_keep_alive();
    //     });

    //     let mut reader = TcpStream::connect("127.0.0.1:8072").unwrap();

    //     match read_message(&mut reader) {
    //         Ok(_s) => assert_eq!(_s.payload, output.payload),
    //         Err(_err) => assert_eq!(69, 420),
    //     }
    // }

    // fn mock_tpc_read_message_keep_alive() {
    //     let listener: TcpListener = TcpListener::bind("127.0.0.1:8072").unwrap();
    //     for stream in listener.incoming() {
    //         match stream {
    //             Ok(mut stream) => {
    //                 println!("New connection: {}", stream.peer_addr().unwrap());
    //                 let input: [u8; 4] = [0, 0, 0, 0];
    //                 stream.write(&input).unwrap();
    //             }
    //             Err(e) => {
    //                 println!("Error: {}", e);
    //                 /* connection failed */
    //             }
    //         }
    //     }
    // }

    // #[test]
    // fn test_read_message_normal() {
    //     let output_payload: Vec<u8> = vec![1, 2, 3, 4];
    //     let output = Message {
    //         id: MESSAGE_HAVE,
    //         payload: output_payload,
    //     };
    //     thread::spawn(move || {
    //         mock_tpc_read_message_normal();
    //     });

    //     let mut reader = TcpStream::connect("127.0.0.1:8057").unwrap();
    //     match read_message(&mut reader) {
    //         Ok(_s) => assert_eq!(_s.id, output.id),
    //         Err(_err) => assert_eq!(69, 420),
    //     }
    // }

    // fn mock_tpc_read_message_normal() {
    //     let listener: TcpListener = TcpListener::bind("127.0.0.1:8057").unwrap();
    //     for stream in listener.incoming() {
    //         match stream {
    //             Ok(mut stream) => {
    //                 println!("New connection: {}", stream.peer_addr().unwrap());
    //                 let input: [u8; 9] = [0, 0, 0, 5, 4, 1, 2, 3, 4];
    //                 stream.write(&input).unwrap();
    //             }
    //             Err(e) => {
    //                 println!("Error: {}", e);
    //                 /* connection failed */
    //             }
    //         }
    //     }
    // }

    #[test]
    fn test_string_choke() {
        let test_message = Message {
            id: MESSAGE_CHOKE,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Choke [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_unchoke() {
        let test_message = Message {
            id: MESSAGE_UNCHOKE,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Unchoke [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_interested() {
        let test_message = Message {
            id: MESSAGE_INTERESTED,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Interested [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_not_interested() {
        let test_message = Message {
            id: MESSAGE_NOT_INTERESTED,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("NotInterested [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_have() {
        let test_message = Message {
            id: MESSAGE_HAVE,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Have [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_bitfield() {
        let test_message = Message {
            id: MESSAGE_BITFIELD,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Bitfield [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_request() {
        let test_message = Message {
            id: MESSAGE_REQUEST,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Request [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_piece() {
        let test_message = Message {
            id: MESSAGE_PIECE,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Piece [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_cancel() {
        let test_message = Message {
            id: MESSAGE_CANCEL,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Cancel [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }

    #[test]
    fn test_string_unknown() {
        let test_message = Message {
            id: 69,
            payload: vec![1, 2, 3],
        };
        let test_string = String::from("Unknown#69 [3]");
        let output_string = string(&test_message);
        assert_eq!(test_string, output_string);
    }
}
