use anchor_lang::prelude::*;

use crate::error::OrgError;
use crate::state::{Membership, MembershipStatus, Organization, Role};

pub const MAX_SLUG_LEN: usize = 32;
pub const MAX_NAME_LEN: usize = 96;

trait CheckedSlug {
    fn checked_slug(&self) -> Result<&str>;
}

impl CheckedSlug for String {
    fn checked_slug(&self) -> Result<&str> {
        require!(
            (1..=MAX_SLUG_LEN).contains(&self.len())
                && self
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            OrgError::SlugInvalid
        );
        Ok(self)
    }
}

#[derive(Accounts)]
#[instruction(slug: String)]
pub struct InitializeOrganization<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Organization::INIT_SPACE,
        // Validated inside the seed: PDA derivation runs before the handler and panics on >32 bytes.
        // Method-call form keeps the IDL seed as the `slug` arg.
        seeds = [b"organization", slug.checked_slug()?.as_bytes()],
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
    require!(!name.is_empty(), OrgError::NameEmpty);
    require!(name.len() <= MAX_NAME_LEN, OrgError::NameTooLong);
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
