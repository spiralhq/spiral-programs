use anchor_lang::prelude::*;

use crate::error::FilmError;
use crate::state::Film;

#[derive(Accounts)]
pub struct DeactivateReplica<'info> {
    #[account(mut, has_one = submitter)]
    pub film: Account<'info, Film>,
    pub submitter: Signer<'info>,
}

pub fn handle_deactivate_replica(ctx: Context<DeactivateReplica>, index: u8) -> Result<()> {
    let film = &mut ctx.accounts.film;
    require!(index < film.replica_count, FilmError::InvalidReplicaIndex);
    film.replicas[index as usize].active = false;
    Ok(())
}
