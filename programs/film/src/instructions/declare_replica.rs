use anchor_lang::prelude::*;

use crate::error::FilmError;
use crate::state::{Film, ReplicaDeclaration, MAX_REPLICAS};

#[derive(Accounts)]
pub struct DeclareReplica<'info> {
    #[account(mut, has_one = submitter)]
    pub film: Account<'info, Film>,
    pub submitter: Signer<'info>,
}

pub fn handle_declare_replica(
    ctx: Context<DeclareReplica>,
    peer_id: [u8; 38],
    peer_id_len: u8,
) -> Result<()> {
    let film = &mut ctx.accounts.film;
    require!(
        (film.replica_count as usize) < MAX_REPLICAS,
        FilmError::ReplicaCapacityExceeded
    );

    let now = Clock::get()?.unix_timestamp;
    let index = film.replica_count as usize;
    film.replicas[index] = ReplicaDeclaration {
        peer_id,
        peer_id_len,
        declared_at: now,
        active: true,
    };
    film.replica_count = film.replica_count.checked_add(1).unwrap();

    Ok(())
}
