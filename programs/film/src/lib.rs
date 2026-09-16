pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use error::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu");

#[program]
pub mod film {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub fn register_film(
        ctx: Context<RegisterFilm>,
        cid: [u8; 64],
        cid_len: u8,
        byte_size: u64,
        chunking_profile: u8,
        title: String,
    ) -> Result<()> {
        handle_register_film(ctx, cid, cid_len, byte_size, chunking_profile, title)
    }

    pub fn declare_replica(
        ctx: Context<DeclareReplica>,
        peer_id: [u8; 38],
        peer_id_len: u8,
    ) -> Result<()> {
        handle_declare_replica(ctx, peer_id, peer_id_len)
    }

    pub fn deactivate_replica(ctx: Context<DeactivateReplica>, index: u8) -> Result<()> {
        handle_deactivate_replica(ctx, index)
    }

    pub fn update_access_policy(
        ctx: Context<UpdateAccessPolicy>,
        new_policy: AccessPolicy,
    ) -> Result<()> {
        handle_update_access_policy(ctx, new_policy)
    }
}
