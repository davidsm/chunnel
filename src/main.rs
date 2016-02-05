extern crate ssh2;
extern crate nix;

use std::net::{TcpStream, TcpListener};
use std::path::Path;
use std::io;
use std::io::{Read, Write};
use std::cmp;
use ssh2::{Session, Error, Channel};
use nix::sys::select::{select, FdSet};
use nix::sys::time::TimeVal;
use std::os::unix::io::AsRawFd;


fn eagain_error(sess: &Session) -> bool {
    match Error::last_error(sess) {
        Some(err) => {
            match err.code() {
                -37 => true,
                _ => false
            }
        },
        None => false
    }
}


fn handle_stream_read(session: &Session, stream: &mut TcpStream,
                      channel: &mut Channel, mut buf: &mut [u8])
                      -> Result<usize, io::Error> {
    match stream.read(&mut buf) {
        Ok(bytes_read) => {
            println!("Read {} bytes from stream", bytes_read);
            let mut bytes_written = 0;
            while bytes_written < bytes_read {
                match channel.write(&buf[bytes_written..bytes_read]) {
                    Ok(bytes) => {
                        bytes_written += bytes;
                    },
                    Err(e) => {
                        println!("Error when writing to channel: {}", e);
                        return Err(e);
                    }
                }
            }
            Ok(bytes_written)
        },
        Err(e) => {
            println!("Error reading from stream: {}", e);
            Err(e)
        }
    }
}

fn handle_channel_read(session: &Session, stream: &mut TcpStream,
                       channel: &mut Channel, mut buf: &mut [u8])
                       -> Result<usize, io::Error> {
    println!("Read from channel");
    match channel.read(&mut buf) {
        Ok(bytes_read) => {
            println!("Read {} bytes from channel", bytes_read);
            match stream.write_all(&buf[0..bytes_read]) {
                Ok(_) => {
                    Ok(bytes_read)
                },
                Err(e) => {
                    println!("Error writing to stream: {}", e);
                    return Err(e)
                }
            }
        },
        Err(e) => {
            println!("Error when reading from channel: {}", e);
            Err(e)
        }
    }
}

fn main() {
    // Connect to the local SSH server
    let tcp = TcpStream::connect("192.168.1.17:22").expect("Could not connect to server");
    let srcport = tcp.local_addr().unwrap().port();
    let mut sess = Session::new().expect("Could not open session");
    println!("Handshaking");
    sess.handshake(&tcp).expect("Could not complete handshake");

    println!("Authenticating");
    sess.userauth_pubkey_file("pi", None, Path::new("/home/david/.ssh/id_rsa"), None).unwrap();

    // Make sure we succeeded
    assert!(sess.authenticated());
    println!("Opening TCP connection from remote host");
    let mut channel = sess.channel_direct_tcpip("127.0.0.1", 22, Some(("127.0.0.1", srcport)))
        .expect("Could not make direct TCP/IP connection");

    sess.set_blocking(false);

    let listener = TcpListener::bind("127.0.0.1:2020").unwrap();
    println!("Listening on port 2020");
    let (mut stream, _) = listener.accept().unwrap();
    println!("Got a connection");

    let mut buf = [0u8; 16384];

    let mut fd_set = FdSet::new();
    let stream_fd = stream.as_raw_fd();
    let channel_fd = tcp.as_raw_fd();

    loop {
        fd_set.clear();
        fd_set.insert(stream_fd);
        fd_set.insert(channel_fd);
        let mut tv = TimeVal::milliseconds(100);
        match select(cmp::max(stream_fd, channel_fd) + 1,
                     Some(&mut fd_set), None, None, &mut tv) {
            Ok(0) => { }
            Ok(_) => {
                if fd_set.contains(stream_fd) {
                    match handle_stream_read(&sess, &mut stream,
                                             &mut channel, &mut buf) {
                        Ok(0) => break,
                        Ok(bytes) => println!("Wrote {} bytes to channel", bytes),
                        Err(_) => break
                    }
                }
                if fd_set.contains(channel_fd) {
                    match handle_channel_read(&sess, &mut stream,
                                              &mut channel, &mut buf) {
                        Ok(0) => break,
                        Ok(bytes) => println!("Wrote {} bytes to stream", bytes),
                        Err(_) => break
                    }
                }
            },
            Err(_) => break
        }
    }
}
