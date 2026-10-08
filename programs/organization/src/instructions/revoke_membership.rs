use anchor_lang::prelude::*;

use crate::authz::require_role;
use crate::error::OrgError;
use crate::events::MembershipRevoked;
use crate::state::{Membership, MembershipStatus, Organization, Role};

#[event_cpi]
#[derive(Accounts)]
pub struct RevokeMembership<'info> {
    pub authority: Signer<'info>,
    pub organization: Account<'info, Organization>,
    #[account(
        seeds = [b"membership", organization.key().as_ref(), authority.key().as_ref()],
        bump = admin_membership.bump,
    )]
    pub admin_membership: Account<'info, Membership>,
    /// CHECK: target member pubkey, used only to derive `membership`'s PDA
    pub member: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership", organization.key().as_ref(), member.key().as_ref()],
        bump = membership.bump,
    )]
    pub membership: Account<'info, Membership>,
}

pub fn handle_revoke_membership(ctx: Context<RevokeMembership>) -> Result<()> {
    require_role(
        &ctx.accounts.admin_membership,
        &ctx.accounts.organization.key(),
        &ctx.accounts.authority.key(),
        &[Role::Admin],
    )?;

    let membership = &mut ctx.accounts.membership;
    require!(
        membership.status == MembershipStatus::Active,
        OrgError::MembershipNotActive
    );
    require_keys_neq!(
        membership.member,
        ctx.accounts.organization.authority,
        OrgError::AuthorityProtected
    );

    let now = Clock::get()?.unix_timestamp;
    membership.status = MembershipStatus::Revoked;
    membership.revoked_at = now;

    emit_cpi!(MembershipRevoked {
        organization: membership.organization,
        member: membership.member,
        revoked_at: now,
    });

    Ok(())
}
