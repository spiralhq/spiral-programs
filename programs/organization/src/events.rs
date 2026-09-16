use anchor_lang::prelude::*;

#[event]
pub struct MembershipRevoked {
    pub organization: Pubkey,
    pub member: Pubkey,
    pub revoked_at: i64,
}
