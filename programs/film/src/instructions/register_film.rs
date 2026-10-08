use anchor_lang::prelude::*;
use solana_sha256_hasher::hash;

use crate::error::FilmError;
use crate::events::FilmRegistered;
use crate::state::{AccessPolicy, Film, ReplicaDeclaration, MAX_REPLICAS};

#[event_cpi]
#[derive(Accounts)]
#[instruction(cid: [u8; 64], cid_len: u8)]
pub struct RegisterFilm<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    #[account(mut)]
    pub organization: Account<'info, organization::Organization>,
    pub membership: Account<'info, organization::Membership>,
    #[account(
        init,
        payer = submitter,
        space = 8 + Film::INIT_SPACE,
        seeds = [
            b"film",
            organization.key().as_ref(),
            &hash(&cid[..(cid_len as usize).min(64)]).to_bytes()[..32],
        ],
        bump,
    )]
    pub film: Account<'info, Film>,
    pub organization_program: Program<'info, organization::program::Organization>,
    pub system_program: Program<'info, System>,
}

pub fn handle_register_film(
    ctx: Context<RegisterFilm>,
    cid: [u8; 64],
    cid_len: u8,
    byte_size: u64,
    chunking_profile: u8,
    title: String,
) -> Result<()> {
    require!(cid_len == 36, FilmError::InvalidCidLength);
    require!(cid[0] == 0x01, FilmError::InvalidCidPrefix);
    require!(
        cid[1] == 0x70 || cid[1] == 0x55,
        FilmError::InvalidCidPrefix
    );
    require!(cid[2] == 0x12, FilmError::InvalidCidPrefix);
    require!(cid[3] == 0x20, FilmError::InvalidCidPrefix);

    let cpi_program_id = ctx.accounts.organization_program.key();
    let cpi_accounts = organization::cpi::accounts::AssertCanSubmit {
        organization: ctx.accounts.organization.to_account_info(),
        membership: ctx.accounts.membership.to_account_info(),
        member: ctx.accounts.submitter.to_account_info(),
    };
    organization::cpi::assert_can_submit(CpiContext::new(cpi_program_id, cpi_accounts))?;

    let now = Clock::get()?.unix_timestamp;

    let film = &mut ctx.accounts.film;
    film.organization = ctx.accounts.organization.key();
    film.submitter = ctx.accounts.submitter.key();
    film.cid = cid;
    film.cid_len = cid_len;
    film.byte_size = byte_size;
    film.chunking_profile = chunking_profile;
    film.title = title;
    film.submitted_at = now;
    film.access_policy = AccessPolicy::Public;
    film.replicas = [ReplicaDeclaration::default(); MAX_REPLICAS];
    film.replica_count = 0;
    film.schema_version = 1;
    film.bump = ctx.bumps.film;

    emit_cpi!(FilmRegistered {
        organization: film.organization,
        submitter: film.submitter,
        cid: film.cid,
        cid_len: film.cid_len,
        byte_size: film.byte_size,
        chunking_profile: film.chunking_profile,
        submitted_at: film.submitted_at,
    });

    Ok(())
}
