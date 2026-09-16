use anchor_lang::prelude::*;

#[error_code]
pub enum FilmError {
    #[msg("CID is truncated or exceeds the Spiral Chunking Profile v1 length")]
    InvalidCidLength = 5,
    #[msg("CID version/codec is outside the Spiral Chunking Profile v1")]
    InvalidCidPrefix,
    #[msg("Replica capacity (8) exceeded")]
    ReplicaCapacityExceeded,
    #[msg("Replica index is out of range")]
    InvalidReplicaIndex,
}
