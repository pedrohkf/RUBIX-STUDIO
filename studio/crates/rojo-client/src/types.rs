//! Tipos do protocolo do Rojo, definidos por nós a partir de docs/rojo-protocol.md.
//!
//! Não copiamos `interface.rs` do Rojo (MPL-2.0): só os nomes dos campos no fio, que são
//! parte da interface. Os valores (`Variant`, `Ref`) vêm do `rbx_types` (MIT).

use std::collections::{HashMap, HashSet};
use std::fmt;

use rbx_types::{Ref, Variant};
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

use crate::error::{Error, Result};

/// Identifica uma execução do servidor. Se mudar, o servidor reiniciou.
///
/// Aceita texto (UUID legível, o caso esperado com msgpack legível) ou 16 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = SessionId;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "um UUID em texto ou 16 bytes")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<SessionId, E> {
                Ok(SessionId(v.to_owned()))
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> std::result::Result<SessionId, E> {
                if v.len() != 16 {
                    return Err(E::custom("SessionId binário deveria ter 16 bytes"));
                }
                let hex: String = v.iter().map(|b| format!("{b:02x}")).collect();
                Ok(SessionId(hex))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// Resposta de `GET /api/rojo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub session_id: SessionId,
    pub server_version: String,
    pub protocol_version: u64,
    pub project_name: String,
    #[serde(default)]
    pub expected_place_ids: Option<HashSet<u64>>,
    #[serde(default)]
    pub unexpected_place_ids: Option<HashSet<u64>>,
    #[serde(default)]
    pub game_id: Option<u64>,
    #[serde(default)]
    pub place_id: Option<u64>,
    pub root_instance_id: Ref,
}

impl ServerInfo {
    /// Falha com erro claro se o protocolo do servidor não for o que entendemos.
    pub fn check_protocol(&self) -> Result<()> {
        if self.protocol_version == crate::error::SUPPORTED_PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(Error::ProtocolMismatch {
                server_version: self.server_version.clone(),
                server_protocol: self.protocol_version,
            })
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    #[serde(default)]
    pub ignore_unknown_instances: bool,
}

/// Uma Instance como o servidor a envia (chaves em PascalCase).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WireInstance {
    pub id: Ref,
    pub parent: Ref,
    pub name: String,
    pub class_name: String,
    #[serde(default)]
    pub properties: HashMap<String, Variant>,
    #[serde(default)]
    pub children: Vec<Ref>,
    #[serde(default)]
    pub metadata: Option<InstanceMetadata>,
}

/// Resposta de `GET /api/read/{ids}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResponse {
    pub session_id: SessionId,
    pub message_cursor: u32,
    pub instances: HashMap<Ref, WireInstance>,
}

/// Mudanças em uma Instance existente. Propriedade com valor nulo = removida.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    pub id: Ref,
    #[serde(default)]
    pub changed_name: Option<String>,
    #[serde(default)]
    pub changed_class_name: Option<String>,
    #[serde(default)]
    pub changed_properties: HashMap<String, Option<Variant>>,
    #[serde(default)]
    pub changed_metadata: Option<InstanceMetadata>,
}

/// Um patch: o que mudou no servidor desde o patch anterior.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeMessage {
    #[serde(default)]
    pub removed: Vec<Ref>,
    #[serde(default)]
    pub added: HashMap<Ref, WireInstance>,
    #[serde(default)]
    pub updated: Vec<InstanceUpdate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesBody {
    pub message_cursor: u32,
    pub messages: Vec<SubscribeMessage>,
}

/// Cada frame binário do WebSocket `/api/socket/{cursor}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocketPacket {
    pub session_id: SessionId,
    pub packet_type: String,
    pub body: MessagesBody,
}

/// Decodifica msgpack do Rojo (structs como mapa, modo legível: `Ref` como texto hex).
pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    let mut deserializer = rmp_serde::Deserializer::new(bytes).with_human_readable();
    T::deserialize(&mut deserializer).map_err(|err| Error::Decode(err.to_string()))
}
