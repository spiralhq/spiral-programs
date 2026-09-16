use anchor_lang::prelude::*;

use crate::authz::require_role;
use crate::state::{Membership, Organization, Role};

#[derive(Accounts)]
pub struct AssertCanSubmit<'info> {
    pub organization: Account<'info, Organization>,
    pub membership: Account<'info, Membership>,
    pub member: Signer<'info>,
}

pub fn handle_assert_can_submit(ctx: Context<AssertCanSubmit>) -> Result<()> {
    require_role(
        &ctx.accounts.membership,
        &ctx.accounts.organization.key(),
        &ctx.accounts.member.key(),
        &[Role::Submitter, Role::Admin],
    )
}
