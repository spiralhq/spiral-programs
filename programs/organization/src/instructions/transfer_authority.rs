use anchor_lang::prelude::*;

use crate::state::Organization;

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub organization: Account<'info, Organization>,
    /// CHECK: new authority pubkey, stored directly; no constraints on this account
    pub new_authority: UncheckedAccount<'info>,
}

pub fn handle_transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
    ctx.accounts.organization.authority = ctx.accounts.new_authority.key();
    Ok(())
}
