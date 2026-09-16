use anchor_lang::prelude::*;

use crate::authz::require_role;
use crate::state::{Membership, Organization, Role};

#[derive(Accounts)]
pub struct UpdateRole<'info> {
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

pub fn handle_update_role(ctx: Context<UpdateRole>, new_role: Role) -> Result<()> {
    require_role(
        &ctx.accounts.admin_membership,
        &ctx.accounts.organization.key(),
        &ctx.accounts.authority.key(),
        &[Role::Admin],
    )?;

    ctx.accounts.membership.role = new_role;

    Ok(())
}
