use anchor_lang::prelude::*;

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
