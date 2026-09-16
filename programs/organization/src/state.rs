use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Organization {
    pub authority: Pubkey,
    #[max_len(32)]
    pub slug: String,
    #[max_len(96)]
    pub name: String,
    pub created_at: i64,
    pub member_count: u32,
    pub schema_version: u8,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Membership {
    pub organization: Pubkey,
    pub member: Pubkey,
    pub role: Role,
    pub status: MembershipStatus,
    pub granted_by: Pubkey,
    pub granted_at: i64,
    pub revoked_at: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum Role {
    Reader,
    Submitter,
    Admin,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum MembershipStatus {
    Active,
    Revoked,
}
