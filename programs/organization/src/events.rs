use anchor_lang::prelude::*;

use crate::state::Role;

#[event]
pub struct MembershipRevoked {
    pub organization: Pubkey,
    pub member: Pubkey,
    pub revoked_at: i64,
}

#[event]
pub struct MembershipGranted {
    pub organization: Pubkey,
    pub member: Pubkey,
    pub role: Role,
    pub granted_by: Pubkey,
    pub granted_at: i64,
}

#[event]
pub struct MembershipRoleUpdated {
    pub organization: Pubkey,
    pub member: Pubkey,
    pub authority: Pubkey,
    pub old_role: Role,
    pub new_role: Role,
}

#[event]
pub struct AuthorityTransferred {
    pub organization: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}
