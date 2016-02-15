use std::path::{Path, PathBuf};
use std::net::TcpStream;
use ssh2;

use error::{SSHError, Result};
use tunnel::Tunnel;

pub struct AuthDetails<'p> {
    user: String,
    authentication: AuthMethod<'p>
}

impl<'p> AuthDetails<'p> {
    pub fn new(user: String, authentication: AuthMethod<'p>) -> AuthDetails<'p> {
        AuthDetails {
            user: user,
            authentication: authentication
        }
    }
}

pub enum AuthMethod<'p> {
    Password(&'p str),
    KeyFile(&'p Path),
    Agent
}

enum AuthData {
    Password(String),
    KeyFile(PathBuf)
}

pub struct SSHSession<'s> {
    host: String,
    port: u16,
    socket: TcpStream,
    session: ssh2::Session,
    via: Option<&'s SSHSession<'s>>,
    user: String,
    auth_data: AuthData
}

impl<'s> SSHSession<'s> {
    pub fn connect(host: &str, port: u16, auth_details: AuthDetails,
                   via: Option<&'s SSHSession<'s>>) -> Result<SSHSession<'s>> {
        let mut session = try!(ssh2::Session::new().ok_or(SSHError::Whatever));

        let socket = match via {
            None => try!(TcpStream::connect((host, port)).or(Err(SSHError::Whatever))),
            Some(other_session) => unimplemented!()
        };

        try!(session.handshake(&socket));
        let auth_data = match auth_details.authentication {
            AuthMethod::Password(_) => unimplemented!(),
            AuthMethod::KeyFile(key_path) => try!(authenticate_with_key(&mut session,
                                                                        &auth_details.user,
                                                                        key_path)),
            AuthMethod::Agent => unimplemented!()
        };

        Ok(SSHSession {
            host: host.to_owned(),
            port: port,
            socket: socket,
            session: session,
            via: via,
            user: auth_details.user,
            auth_data: auth_data
        })
    }

    pub fn tunnel_to(&'s self, host: &str, port: u16, listen_port: u16) -> Result<Tunnel> {
        let channel = try!(self.connect_to(host, port));
        let tunnel = try!(Tunnel::establish(listen_port, self, channel));
        Ok(tunnel)
    }

    fn connect_to(&'s self, host: &str, port: u16) -> Result<ssh2::Channel<'s>> {
        self.session.channel_direct_tcpip(host, port, None)
            .map_err(|e| SSHError::from(e))
    }

    pub fn socket(&self) -> &TcpStream {
        &self.socket
    }

    pub fn set_blocking(&self, block: bool) {
        self.session.set_blocking(block);
    }

}

fn authenticate_with_key(session: &mut ssh2::Session, user: &str,
                         key_path: &Path) -> Result<AuthData> {
    // Reminder: does not support passphrase for key
    try!(session.userauth_pubkey_file(user, None, key_path, None));
    Ok(AuthData::KeyFile(key_path.to_owned()))
}
