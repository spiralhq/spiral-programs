use anchor_lang::prelude::*;
use organization::{program::Organization as OrganizationProgram, Membership, OrgError, Role};

use crate::state::Film;

pub fn require_film_manager<'info>(
    authority: &Signer<'info>,
    organization: &Account<'info, organization::Organization>,
    membership: &Account<'info, Membership>,
    organization_program: &Program<'info, OrganizationProgram>,
    film: &Film,
) -> Result<()> {
    let cpi_accounts = organization::cpi::accounts::AssertCanSubmit {
        organization: organization.to_account_info(),
        membership: membership.to_account_info(),
        member: authority.to_account_info(),
    };
    organization::cpi::assert_can_submit(CpiContext::new(
        organization_program.key(),
        cpi_accounts,
    ))?;
    if authority.key() != film.submitter {
        require!(membership.role == Role::Admin, OrgError::InsufficientRole);
    }
    Ok(())
}
