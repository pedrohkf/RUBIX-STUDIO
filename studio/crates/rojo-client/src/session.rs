//! Sessão ao vivo: leitura inicial + WebSocket `/api/socket/{cursor}` aplicando patches.
//!
//! A engine chama `poll` a cada quadro. Nunca bloqueia: sem patch novo devolve `Poll::Idle`.
//! Se o servidor reiniciar (sessão nova) ou o socket cair, `poll` devolve erro e o chamador
//! recria a `Session` (releitura completa, sempre segura).

use std::io::ErrorKind;
use std::net::TcpStream;
use std::time::Duration;

use dom::WeakDom;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::client::{RojoClient, Snapshot};
use crate::error::{Error, Result};
use crate::sync::apply_message;
use crate::types::{ServerInfo, SocketPacket, decode};

pub struct Session {
    pub info: ServerInfo,
    pub dom: WeakDom,
    cursor: u32,
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

/// Resultado de um `poll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// Nada novo.
    Idle,
    /// Foram aplicados `patches` patches; a árvore mudou.
    Patched { patches: usize },
}

impl Session {
    /// Handshake, leitura da árvore e abertura do WebSocket a partir do cursor lido.
    pub fn connect(client: &RojoClient) -> Result<Self> {
        let Snapshot {
            info,
            dom,
            message_cursor,
        } = client.snapshot()?;
        let host = client
            .base
            .strip_prefix("http://")
            .ok_or_else(|| Error::Socket(format!("só http:// é suportado: {}", client.base)))?;
        let stream = TcpStream::connect(host).map_err(|err| Error::Socket(err.to_string()))?;
        stream
            .set_nodelay(true)
            .map_err(|err| Error::Socket(err.to_string()))?;
        let url = format!("ws://{host}/api/socket/{message_cursor}");
        let (socket, _) = tungstenite::client(url, MaybeTlsStream::Plain(stream))
            .map_err(|err| Error::Socket(err.to_string()))?;
        // Leitura não bloqueante só depois do upgrade (o handshake precisa bloquear).
        if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
            stream
                .set_nonblocking(true)
                .map_err(|err| Error::Socket(err.to_string()))?;
        }
        Ok(Self {
            info,
            dom,
            cursor: message_cursor,
            socket,
        })
    }

    pub fn cursor(&self) -> u32 {
        self.cursor
    }

    /// Lê tudo o que já chegou e aplica na árvore.
    pub fn poll(&mut self) -> Result<Poll> {
        let mut patches = 0usize;
        loop {
            match self.socket.read() {
                Ok(Message::Binary(bytes)) => {
                    let packet: SocketPacket = decode(&bytes)?;
                    if packet.session_id != self.info.session_id {
                        return Err(Error::SessionChanged);
                    }
                    for message in &packet.body.messages {
                        apply_message(&mut self.dom, message)?;
                        patches += 1;
                    }
                    self.cursor = packet.body.message_cursor;
                }
                Ok(Message::Close(_)) => {
                    return Err(Error::Socket("servidor fechou o WebSocket".into()));
                }
                Ok(_) => {} // ping/pong/texto: o tungstenite responde ao ping sozinho
                Err(tungstenite::Error::Io(err))
                    if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    break;
                }
                Err(err) => return Err(Error::Socket(err.to_string())),
            }
        }
        Ok(if patches == 0 {
            Poll::Idle
        } else {
            Poll::Patched { patches }
        })
    }

    /// Chama `poll` até aparecer patch ou estourar `timeout` (útil em testes e na CLI).
    pub fn wait_patch(&mut self, timeout: Duration) -> Result<Poll> {
        let start = std::time::Instant::now();
        loop {
            let result = self.poll()?;
            if result != Poll::Idle || start.elapsed() >= timeout {
                return Ok(result);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
