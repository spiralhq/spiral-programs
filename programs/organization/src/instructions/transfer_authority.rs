use anchor_lang::prelude::*;

use crate::authz::require_role;
use crate::events::AuthorityTransferred;
use crate::state::{Membership, Organization, Role};

#[event_cpi]
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub organization: Account<'info, Organization>,
    /// CHECK: new authority pubkey; must hold an active Admin `new_authority_membership`
    pub new_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"membership", organization.key().as_ref(), new_authority.key().as_ref()],
        bump = new_authority_membership.bump,
    )]
    pub new_authority_membership: Account<'info, Membership>,
}

pub fn handle_transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
    require_role(
        &ctx.accounts.new_authority_membership,
        &ctx.accounts.organization.key(),
        &ctx.accounts.new_authority.key(),
        &[Role::Admin],
    )?;
    let old_authority = ctx.accounts.organization.authority;
    ctx.accounts.organization.authority = ctx.accounts.new_authority.key();

    emit_cpi!(AuthorityTransferred {
        organization: ctx.accounts.organization.key(),
        old_authority,
        new_authority: ctx.accounts.new_authority.key(),
    });
    Ok(())
}
