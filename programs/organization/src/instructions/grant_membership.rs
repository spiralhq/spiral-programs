use anchor_lang::prelude::*;

use crate::authz::require_role;
use crate::state::{Membership, MembershipStatus, Organization, Role};

#[derive(Accounts)]
pub struct GrantMembership<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub organization: Account<'info, Organization>,
    #[account(
        seeds = [b"membership", organization.key().as_ref(), authority.key().as_ref()],
        bump = admin_membership.bump,
    )]
    pub admin_membership: Account<'info, Membership>,
    /// CHECK: pubkey of the member being granted access; does not need to sign
    pub member: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Membership::INIT_SPACE,
        seeds = [b"membership", organization.key().as_ref(), member.key().as_ref()],
        bump,
    )]
    pub membership: Account<'info, Membership>,
    pub system_program: Program<'info, System>,
}

pub fn handle_grant_membership(ctx: Context<GrantMembership>, role: Role) -> Result<()> {
    require_role(
        &ctx.accounts.admin_membership,
        &ctx.accounts.organization.key(),
        &ctx.accounts.authority.key(),
        &[Role::Admin],
    )?;

    let now = Clock::get()?.unix_timestamp;
    let membership = &mut ctx.accounts.membership;
    membership.organization = ctx.accounts.organization.key();
    membership.member = ctx.accounts.member.key();
    membership.role = role;
    membership.status = MembershipStatus::Active;
    membership.granted_by = ctx.accounts.authority.key();
    membership.granted_at = now;
    membership.revoked_at = 0;
    membership.bump = ctx.bumps.membership;

    ctx.accounts.organization.member_count = ctx
        .accounts
        .organization
        .member_count
        .checked_add(1)
        .unwrap();

    Ok(())
}
