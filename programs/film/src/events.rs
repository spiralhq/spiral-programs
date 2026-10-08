use anchor_lang::prelude::*;

use crate::state::AccessPolicy;

#[event]
pub struct FilmRegistered {
    pub organization: Pubkey,
    pub submitter: Pubkey,
    pub cid: [u8; 64],
    pub cid_len: u8,
    pub byte_size: u64,
    pub chunking_profile: u8,
    pub submitted_at: i64,
}

#[event]
pub struct ReplicaDeclared {
    pub film: Pubkey,
    pub authority: Pubkey,
    pub peer_id: [u8; 38],
    pub peer_id_len: u8,
    pub slot: u8,
    pub reactivated: bool,
    pub replaced_peer_id: Option<[u8; 38]>,
}

#[event]
pub struct ReplicaDeactivated {
    pub film: Pubkey,
    pub authority: Pubkey,
    pub slot: u8,
    pub peer_id: [u8; 38],
    pub was_active: bool,
}

#[event]
pub struct AccessPolicyUpdated {
    pub film: Pubkey,
    pub authority: Pubkey,
    pub policy: AccessPolicy,
}
