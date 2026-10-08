use anchor_lang::prelude::*;

#[error_code(offset = 6005)]
pub enum FilmError {
    #[msg("CID is truncated or exceeds the Spiral Chunking Profile v1 length")]
    InvalidCidLength,
    #[msg("CID version/codec is outside the Spiral Chunking Profile v1")]
    InvalidCidPrefix,
    #[msg("Replica capacity (8) exceeded")]
    ReplicaCapacityExceeded,
    #[msg("Replica index is out of range")]
    InvalidReplicaIndex,
    #[msg("PeerID is not a 38-byte Ed25519 libp2p identity")]
    InvalidPeerId,
    #[msg("PeerID already holds an active replica slot")]
    DuplicateReplica,
    #[msg("Title exceeds 160 bytes")]
    TitleTooLong,
    #[msg("CID buffer must be zero after cid_len")]
    CidPaddingNotZero,
    #[msg("Replica count overflowed")]
    ReplicaCountOverflow,
}
