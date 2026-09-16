pub mod authz;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use error::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw");

#[program]
pub mod organization {
    use super::*;

    pub fn initialize_organization(
        ctx: Context<InitializeOrganization>,
        slug: String,
        name: String,
    ) -> Result<()> {
        handle_initialize_organization(ctx, slug, name)
    }

    pub fn grant_membership(ctx: Context<GrantMembership>, role: Role) -> Result<()> {
        handle_grant_membership(ctx, role)
    }

    pub fn update_role(ctx: Context<UpdateRole>, new_role: Role) -> Result<()> {
        handle_update_role(ctx, new_role)
    }

    pub fn revoke_membership(ctx: Context<RevokeMembership>) -> Result<()> {
        handle_revoke_membership(ctx)
    }

    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        handle_transfer_authority(ctx)
    }

    pub fn assert_can_submit(ctx: Context<AssertCanSubmit>) -> Result<()> {
        handle_assert_can_submit(ctx)
    }
}
