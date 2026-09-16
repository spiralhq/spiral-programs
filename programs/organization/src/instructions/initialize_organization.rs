use anchor_lang::prelude::*;

use crate::state::{Membership, MembershipStatus, Organization, Role};

#[derive(Accounts)]
#[instruction(slug: String)]
pub struct InitializeOrganization<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Organization::INIT_SPACE,
        seeds = [b"organization", slug.as_bytes()],
        bump,
    )]
    pub organization: Account<'info, Organization>,
    #[account(
        init,
        payer = authority,
        space = 8 + Membership::INIT_SPACE,
        seeds = [b"membership", organization.key().as_ref(), authority.key().as_ref()],
        bump,
    )]
    pub membership: Account<'info, Membership>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_organization(
    ctx: Context<InitializeOrganization>,
    slug: String,
    name: String,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let organization = &mut ctx.accounts.organization;
    organization.authority = ctx.accounts.authority.key();
    organization.slug = slug;
    organization.name = name;
    organization.created_at = now;
    organization.member_count = 1;
    organization.schema_version = 1;
    organization.bump = ctx.bumps.organization;

    let membership = &mut ctx.accounts.membership;
    membership.organization = organization.key();
    membership.member = ctx.accounts.authority.key();
    membership.role = Role::Admin;
    membership.status = MembershipStatus::Active;
    membership.granted_by = ctx.accounts.authority.key();
    membership.granted_at = now;
    membership.revoked_at = 0;
    membership.bump = ctx.bumps.membership;

    Ok(())
}
