use anchor_lang::prelude::*;

use crate::error::OrgError;
use crate::state::{Membership, MembershipStatus, Role};

pub fn require_role(
    membership: &Account<Membership>,
    organization: &Pubkey,
    signer: &Pubkey,
    allowed: &[Role],
) -> Result<()> {
    require!(
        membership.status == MembershipStatus::Active,
        OrgError::MembershipRevoked
    );
    require!(
        allowed.contains(&membership.role),
        OrgError::InsufficientRole
    );
    require_keys_eq!(
        membership.organization,
        *organization,
        OrgError::OrgMismatch
    );
    require_keys_eq!(membership.member, *signer, OrgError::MemberMismatch);
    Ok(())
}
