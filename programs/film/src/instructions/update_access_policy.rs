use anchor_lang::prelude::*;

use crate::state::{AccessPolicy, Film};

#[derive(Accounts)]
pub struct UpdateAccessPolicy<'info> {
    #[account(mut, has_one = submitter)]
    pub film: Account<'info, Film>,
    pub submitter: Signer<'info>,
}

pub fn handle_update_access_policy(
    ctx: Context<UpdateAccessPolicy>,
    new_policy: AccessPolicy,
) -> Result<()> {
    ctx.accounts.film.access_policy = new_policy;
    Ok(())
}
