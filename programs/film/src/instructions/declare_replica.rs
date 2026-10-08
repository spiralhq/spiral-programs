use anchor_lang::prelude::*;
use organization::{program::Organization as OrganizationProgram, Membership, OrgError};

use crate::error::FilmError;
use crate::events::ReplicaDeclared;
use crate::instructions::manage::require_film_manager;
use crate::state::{Film, ReplicaDeclaration, ED25519_PEER_ID_PREFIX, MAX_REPLICAS, PEER_ID_LEN};

#[event_cpi]
#[derive(Accounts)]
pub struct DeclareReplica<'info> {
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

pub fn handle_declare_replica(
    ctx: Context<DeclareReplica>,
    peer_id: [u8; 38],
    peer_id_len: u8,
) -> Result<()> {
    require_film_manager(
        &ctx.accounts.authority,
        &ctx.accounts.organization,
        &ctx.accounts.membership,
        &ctx.accounts.organization_program,
        &ctx.accounts.film,
    )?;
    require!(
        peer_id_len == PEER_ID_LEN && peer_id[..6] == ED25519_PEER_ID_PREFIX,
        FilmError::InvalidPeerId
    );

    let film_key = ctx.accounts.film.key();
    let film = &mut ctx.accounts.film;
    let count = film.replica_count as usize;
    let used = &film.replicas[..count];

    require!(
        !used.iter().any(|r| r.active && r.peer_id == peer_id),
        FilmError::DuplicateReplica
    );
    let (index, reactivated) = match used.iter().position(|r| r.peer_id == peer_id) {
        Some(i) => (i, true),
        None if count < MAX_REPLICAS => (count, false),
        None => (
            used.iter()
                .position(|r| !r.active)
                .ok_or(FilmError::ReplicaCapacityExceeded)?,
            false,
        ),
    };
    let replaced_peer_id = (!reactivated && index < count).then(|| film.replicas[index].peer_id);

    film.replicas[index] = ReplicaDeclaration {
        peer_id,
        peer_id_len,
        declared_at: Clock::get()?.unix_timestamp,
        active: true,
    };
    if index == count {
        film.replica_count = film
            .replica_count
            .checked_add(1)
            .ok_or(FilmError::ReplicaCountOverflow)?;
    }

    emit_cpi!(ReplicaDeclared {
        film: film_key,
        authority: ctx.accounts.authority.key(),
        peer_id,
        peer_id_len,
        slot: index as u8,
        reactivated,
        replaced_peer_id,
    });

    Ok(())
}
