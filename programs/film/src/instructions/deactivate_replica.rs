use anchor_lang::prelude::*;
use organization::{program::Organization as OrganizationProgram, Membership, OrgError};

use crate::error::FilmError;
use crate::events::ReplicaDeactivated;
use crate::instructions::manage::require_film_manager;
use crate::state::Film;

#[event_cpi]
#[derive(Accounts)]
pub struct DeactivateReplica<'info> {
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

pub fn handle_deactivate_replica(ctx: Context<DeactivateReplica>, index: u8) -> Result<()> {
    require_film_manager(
        &ctx.accounts.authority,
        &ctx.accounts.organization,
        &ctx.accounts.membership,
        &ctx.accounts.organization_program,
        &ctx.accounts.film,
    )?;
    let film_key = ctx.accounts.film.key();
    let film = &mut ctx.accounts.film;
    require!(index < film.replica_count, FilmError::InvalidReplicaIndex);
    let replica = &mut film.replicas[index as usize];
    let was_active = replica.active;
    replica.active = false;

    emit_cpi!(ReplicaDeactivated {
        film: film_key,
        authority: ctx.accounts.authority.key(),
        slot: index,
        peer_id: replica.peer_id,
        was_active,
    });

    Ok(())
}
