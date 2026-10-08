use anchor_lang::prelude::*;
use organization::{program::Organization as OrganizationProgram, Membership, OrgError};

use crate::events::AccessPolicyUpdated;
use crate::instructions::manage::require_film_manager;
use crate::state::{AccessPolicy, Film};

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateAccessPolicy<'info> {
    pub authority: Signer<'info>,
    pub organization: Account<'info, organization::Organization>,
    #[account(
        seeds = [b"membership", organization.key().as_ref(), authority.key().as_ref()],
        seeds::program = organization_program.key(),
        bump = membership.bump,
    )]
    pub membership: Account<'info, Membership>,
    pub organization_program: Program<'info, OrganizationProgram>,
    #[account(mut, has_one = organization @ OrgError::OrgMismatch)]
    pub film: Account<'info, Film>,
}

pub fn handle_update_access_policy(
    ctx: Context<UpdateAccessPolicy>,
    new_policy: AccessPolicy,
) -> Result<()> {
    require_film_manager(
        &ctx.accounts.authority,
        &ctx.accounts.organization,
        &ctx.accounts.membership,
        &ctx.accounts.organization_program,
        &ctx.accounts.film,
    )?;
    ctx.accounts.film.access_policy = new_policy;

    emit_cpi!(AccessPolicyUpdated {
        film: ctx.accounts.film.key(),
        authority: ctx.accounts.authority.key(),
        policy: new_policy,
    });

    Ok(())
}
