use anchor_lang::prelude::*;

pub const MAX_REPLICAS: usize = 8;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct ReplicaDeclaration {
    pub peer_id: [u8; 38],
    pub peer_id_len: u8,
    pub declared_at: i64,
    pub active: bool,
}

impl Default for ReplicaDeclaration {
    fn default() -> Self {
        Self {
            peer_id: [0u8; 38],
            peer_id_len: 0,
            declared_at: 0,
            active: false,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum AccessPolicy {
    Public,
    OrganizationOnly,
}

#[account]
#[derive(InitSpace)]
pub struct Film {
    pub organization: Pubkey,
    pub submitter: Pubkey,
    pub cid: [u8; 64],
    pub cid_len: u8,
    pub byte_size: u64,
    pub chunking_profile: u8,
    #[max_len(160)]
    pub title: String,
    pub submitted_at: i64,
    pub access_policy: AccessPolicy,
    pub replicas: [ReplicaDeclaration; MAX_REPLICAS],
    pub replica_count: u8,
    pub schema_version: u8,
    pub bump: u8,
}
