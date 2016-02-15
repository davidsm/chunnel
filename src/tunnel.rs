use std::net::{TcpListener, TcpStream};
use std::io;
use std::io::{Read, Write};
use ssh2;

use session::SSHSession;
use error::{Result, SSHError};

pub struct Tunnel<'s> {
    listener: TcpListener,
    session: &'s SSHSession<'s>,
    channel: ssh2::Channel<'s>
}

impl<'s> Tunnel<'s> {
    pub fn establish(listen_port: u16, session: &'s SSHSession, channel: ssh2::Channel<'s>) -> Result<Tunnel<'s>> {
        let listener = try!(TcpListener::bind(("127.0.0.1", listen_port))
                            .or(Err(SSHError::Whatever)));
        Ok(Tunnel {
            listener: listener,
            session: session,
            channel: channel
        })
    }

    pub fn accept(&self) -> Result<TcpStream> {
        let (socket, _) = try!(self.listener.accept().or(Err(SSHError::Whatever)));
        Ok(socket)
    }
}

impl<'s> Read for Tunnel<'s> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.channel.read(buf)
    }
}

impl<'s> Write for Tunnel<'s> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.channel.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.channel.flush()
    }
}
