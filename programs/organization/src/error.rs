use anchor_lang::prelude::*;

#[error_code]
pub enum OrgError {
    #[msg("No membership account exists for the signer (reserved; Anchor reports AccountNotInitialized)")]
    MembershipNotFound,
    #[msg("Signer's membership has been revoked")]
    MembershipRevoked,
    #[msg("Role does not permit this action")]
    InsufficientRole,
    #[msg("Membership belongs to a different organization")]
    OrgMismatch,
    #[msg("Signer does not match membership.member")]
    MemberMismatch,
    #[msg("Slug must be 1-32 bytes of [a-z0-9-]")]
    SlugInvalid,
    #[msg("Name exceeds 96 bytes")]
    NameTooLong,
    #[msg("Target membership is not active")]
    MembershipNotActive,
    #[msg("The organization authority's membership cannot be revoked or changed")]
    AuthorityProtected,
    #[msg("Organization member count overflowed")]
    MemberCountOverflow,
    #[msg("Name must not be empty")]
    NameEmpty,
}
