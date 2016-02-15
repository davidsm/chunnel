extern crate ssh2;
extern crate nix;

use std::net::{TcpStream, TcpListener};
use std::path::Path;
use std::io;
use std::io::{Read, Write};
use std::cmp;
use ssh2::{Session, Error};
use nix::sys::select::{select, FdSet};
use nix::sys::time::TimeVal;
use std::os::unix::io::AsRawFd;

mod session;
use session::{SSHSession, AuthDetails, AuthMethod};

mod tunnel;
mod error;

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

fn handle_read_ready<S, T>(source: &mut S, target: &mut T, mut buf: &mut [u8])
                           -> Result<usize, io::Error> where S: Read, T: Write {
    match source.read(&mut buf) {
        Ok(bytes_read) => {
            let mut bytes_written = 0;
            while bytes_written < bytes_read {
                match target.write(&buf[bytes_written..bytes_read]) {
                    Ok(bytes) => {
                        bytes_written += bytes;
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            Ok(bytes_written)
        },
        Err(e) => {
            Err(e)
        }
    }
}

fn main() {
    let auth_details = AuthDetails::new("pi".to_string(),
                                        AuthMethod::KeyFile(&Path::new("/home/david/.ssh/id_rsa")));

    let session = SSHSession::connect("192.168.1.17", 22,
                                      auth_details, None)
        .expect("Could not connect/authenticate");

    let mut tunnel = session.tunnel_to("127.0.0.1", 22, 2020)
        .expect("Could not make tunneled connection");
    println!("Listening on port 2020");

    session.set_blocking(false);

    let mut stream = tunnel.accept().unwrap();
    println!("Got a connection");

    let mut buf = [0u8; 16384];

    let mut fd_set = FdSet::new();
    let stream_fd = stream.as_raw_fd();
    let channel_fd = session.socket().as_raw_fd();

    loop {
        fd_set.clear();
        fd_set.insert(stream_fd);
        fd_set.insert(channel_fd);
        let mut tv = TimeVal::milliseconds(100);
        match select(cmp::max(stream_fd, channel_fd) + 1,
                     Some(&mut fd_set), None, None, &mut tv) {
            Ok(0) => { },
            Ok(_) => {
                if fd_set.contains(stream_fd) {
                    match handle_read_ready(&mut stream, &mut tunnel, &mut buf) {
                        Ok(0) => break,
                        Ok(bytes) => println!("Wrote {} bytes to channel", bytes),
                        Err(_) => break
                    }
                }
                if fd_set.contains(channel_fd) {
                    match handle_read_ready(&mut tunnel, &mut stream, &mut buf) {
                        Ok(0) => break,
                        Ok(bytes) => println!("Wrote {} bytes to stream", bytes),
                        Err(_) => break
                    }
                }
            },
            Err(_) => break
        }
    }

    // Need to do this, or Session will cause a panic when dropped
    // due to libssh2_session_free returning -37 (EAGAIN)
    session.set_blocking(true);
}
