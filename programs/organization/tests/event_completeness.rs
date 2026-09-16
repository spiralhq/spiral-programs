use organization::{Membership, MembershipRevoked, MembershipStatus, Role};

/// `MembershipRevoked` must be a pure field-for-field subset of `Membership`.
#[test]
fn membership_revoked_event_is_a_subset_of_membership_account() {
    let membership = Membership {
        organization: anchor_lang::prelude::Pubkey::new_unique(),
        member: anchor_lang::prelude::Pubkey::new_unique(),
        role: Role::Submitter,
        status: MembershipStatus::Revoked,
        granted_by: anchor_lang::prelude::Pubkey::new_unique(),
        granted_at: 1_600_000_000,
        revoked_at: 1_700_000_000,
        bump: 254,
    };

    let event = MembershipRevoked {
        organization: membership.organization,
        member: membership.member,
        revoked_at: membership.revoked_at,
    };

    assert_eq!(event.organization, membership.organization);
    assert_eq!(event.member, membership.member);
    assert_eq!(event.revoked_at, membership.revoked_at);
}
