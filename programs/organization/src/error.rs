use anchor_lang::prelude::*;

#[error_code]
pub enum OrgError {
    #[msg("Signer has no active membership in this organization")]
    MembershipNotFound,
    #[msg("Membership has been revoked")]
    MembershipRevoked,
    #[msg("Role does not permit this action")]
    InsufficientRole,
    #[msg("Membership belongs to a different organization")]
    OrgMismatch,
    #[msg("Signer does not match membership.member")]
    MemberMismatch,
}
