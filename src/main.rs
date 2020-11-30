mod bitfield;
mod client;
mod handshake;
mod message;
mod p2p;
mod peers;
mod torrentfile;
mod tracker;
use std::env;
use torrentfile::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_path = &args[1];
    let out_path = &args[2];
    println!(
        "{}",
        r"                                                                                 
        ___  __  ____________   __________  ___  ___  _____  ________
        / _ \/ / / / __/_  __/__/_  __/ __ \/ _ \/ _ \/ __/ |/ /_  __/
       / , _/ /_/ /\ \  / / /___// / / /_/ / , _/ , _/ _//    / / /   
      /_/|_|\____/___/ /_/      /_/  \____/_/|_/_/|_/___/_/|_/ /_/                                                                                                                                           
   "
    );

    let mut torrent_file = open(in_path.to_owned()).unwrap();
    torrent_file.download_to_file(out_path.to_owned()).unwrap();
}
