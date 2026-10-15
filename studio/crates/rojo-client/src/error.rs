use rbx_types::Ref;

/// Versão do protocolo do Rojo que esta engine entende (ver docs/rojo-protocol.md).
pub const SUPPORTED_PROTOCOL_VERSION: u64 = 5;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "O servidor Rojo usa outra versão do protocolo e é incompatível.\n\
         Servidor: versão {server_version}, protocolo {server_protocol}.\n\
         Esta engine entende o protocolo {SUPPORTED_PROTOCOL_VERSION} (Rojo 7.x).\n\
         Use um Rojo com protocolo {SUPPORTED_PROTOCOL_VERSION} ou atualize a engine."
    )]
    ProtocolMismatch {
        server_version: String,
        server_protocol: u64,
    },

    #[error("resposta do servidor Rojo ilegível (msgpack): {0}")]
    Decode(String),

    #[error("falha de rede ao falar com o Rojo: {0}")]
    Network(String),

    #[error("o servidor Rojo respondeu HTTP {status}: {details}")]
    Http { status: u16, details: String },

    #[error("instância {0:?} citada pelo servidor não veio na leitura")]
    MissingInstance(Ref),

    #[error("a raiz {0:?} não foi enviada pelo servidor")]
    MissingRoot(Ref),

    #[error("patch inválido: instância {0:?} já existe")]
    DuplicateInstance(Ref),

    #[error("patch inválido: instância {0:?} não existe")]
    UnknownInstance(Ref),

    #[error("patch inválido: {count} instância(s) adicionada(s) sem pai conhecido")]
    OrphanInstances { count: usize },

    #[error("patch inválido: tentou remover a raiz da árvore")]
    RemovedRoot,

    #[error("falha no WebSocket do Rojo: {0}")]
    Socket(String),

    #[error("a sessão do Rojo mudou (o servidor reiniciou): é preciso reler a árvore")]
    SessionChanged,
}

pub type Result<T> = std::result::Result<T, Error>;
