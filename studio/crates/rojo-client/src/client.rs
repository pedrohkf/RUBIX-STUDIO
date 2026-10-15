//! Transporte HTTP para o `rojo serve` local. Só leitura: `/api/rojo` e `/api/read`.
//!
//! O WebSocket (`/api/socket/{cursor}`) entra no próximo passo; os tipos e a aplicação de
//! patches já estão prontos em `types` e `sync`.

use std::time::Duration;

use dom::WeakDom;

use crate::error::{Error, Result};
use crate::sync::build_dom;
use crate::types::{ReadResponse, ServerInfo, decode};

/// Porta padrão do `rojo serve` (`DEFAULT_PORT` em `src/cli/serve.rs` do Rojo).
pub const DEFAULT_PORT: u16 = 34872;

pub struct RojoClient {
    pub(crate) base: String,
    agent: ureq::Agent,
}

/// Árvore lida do servidor, com o que é preciso para continuar pelo WebSocket.
pub struct Snapshot {
    pub info: ServerInfo,
    pub dom: WeakDom,
    pub message_cursor: u32,
}

impl RojoClient {
    /// `base` sem barra final, por exemplo `http://127.0.0.1:34872`.
    pub fn new(base: impl Into<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(10)))
            .build();
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub fn local() -> Self {
        Self::new(format!("http://127.0.0.1:{DEFAULT_PORT}"))
    }

    fn get(&self, path: &str) -> Result<Vec<u8>> {
        let url = format!("{}{}", self.base, path);
        let mut response = self
            .agent
            .get(&url)
            .call()
            .map_err(|err| Error::Network(err.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|err| Error::Network(err.to_string()))?;
        if status >= 400 {
            return Err(Error::Http {
                status,
                details: String::from_utf8_lossy(&body).into_owned(),
            });
        }
        Ok(body)
    }

    /// `GET /api/rojo` e validação da versão do protocolo.
    pub fn handshake(&self) -> Result<ServerInfo> {
        let bytes = self.get("/api/rojo")?;
        let info: ServerInfo = decode(&bytes)?;
        info.check_protocol()?;
        Ok(info)
    }

    /// Handshake, leitura da árvore inteira e construção do DataModel.
    pub fn snapshot(&self) -> Result<Snapshot> {
        let info = self.handshake()?;
        let root = info.root_instance_id;
        let bytes = self.get(&format!("/api/read/{root}"))?;
        let read: ReadResponse = decode(&bytes)?;
        if read.session_id != info.session_id {
            return Err(Error::SessionChanged);
        }
        let dom = build_dom(&read, root)?;
        Ok(Snapshot {
            info,
            dom,
            message_cursor: read.message_cursor,
        })
    }
}
