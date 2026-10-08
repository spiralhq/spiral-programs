#[path = "../../../test-support/program_bytes.rs"]
mod program_bytes;

use program_bytes::program_bytes;

use {
    anchor_lang::{
        prelude::{AnchorDeserialize, Clock, Discriminator, Pubkey},
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    organization::{Membership, MembershipStatus, Role},
    solana_instruction::error::InstructionError,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_error::TransactionError,
};

const MEMBERSHIP_REVOKED: u32 = 6001;
const INSUFFICIENT_ROLE: u32 = 6002;
const SLUG_INVALID: u32 = 6005;
const NAME_TOO_LONG: u32 = 6006;
const MEMBERSHIP_NOT_ACTIVE: u32 = 6007;
const AUTHORITY_PROTECTED: u32 = 6008;
const MEMBER_COUNT_OVERFLOW: u32 = 6009;
const NAME_EMPTY: u32 = 6010;

fn load_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.set_sysvar(&Clock {
        unix_timestamp: 1_700_000_000,
        ..Default::default()
    });
    svm.add_program(organization::id(), &program_bytes("organization"))
        .unwrap();
    svm
}

#[allow(clippy::result_large_err)]
fn send(svm: &mut LiteSVM, payer: &Keypair, ixs: &[Instruction]) -> TransactionResult {
    svm.expire_blockhash();
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &bh);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx)
}

fn assert_ok(res: &TransactionResult) {
    assert!(
        res.is_ok(),
        "expected success, got: {:?}",
        res.as_ref().err()
    );
}

fn assert_error_code(res: &TransactionResult, expected: u32) {
    match res {
        Ok(_) => panic!("expected failure with error code {expected}, got success"),
        Err(e) => match &e.err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                assert_eq!(*code, expected, "wrong error code");
            }
            other => panic!("expected Custom({expected}), got {other:?}"),
        },
    }
}

fn membership_pda(org: &Pubkey, member: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"membership", org.as_ref(), member.as_ref()],
        &organization::id(),
    )
    .0
}

fn org_address(slug: &str) -> Pubkey {
    Pubkey::try_find_program_address(&[b"organization", slug.as_bytes()], &organization::id())
        .map(|(pda, _)| pda)
        .unwrap_or_else(Pubkey::new_unique)
}

#[allow(clippy::result_large_err)]
fn try_init(svm: &mut LiteSVM, authority: &Keypair, slug: &str, name: &str) -> TransactionResult {
    let org = org_address(slug);
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.to_string(),
            name: name.to_string(),
        }
        .data(),
        organization::accounts::InitializeOrganization {
            authority: authority.pubkey(),
            organization: org,
            membership: membership_pda(&org, &authority.pubkey()),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(svm, authority, &[ix])
}

fn funded(svm: &mut LiteSVM) -> Keypair {
    let k = Keypair::new();
    svm.airdrop(&k.pubkey(), 10_000_000_000).unwrap();
    k
}

fn grant(svm: &mut LiteSVM, admin: &Keypair, org: Pubkey, member: Pubkey, role: Role) {
    assert_ok(&try_grant(svm, admin, org, member, role));
}

#[allow(clippy::result_large_err)]
fn try_grant(
    svm: &mut LiteSVM,
    admin: &Keypair,
    org: Pubkey,
    member: Pubkey,
    role: Role,
) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::GrantMembership { role }.data(),
        organization::accounts::GrantMembership {
            authority: admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &admin.pubkey()),
            member,
            membership: membership_pda(&org, &member),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(svm, admin, &[ix])
}

#[allow(clippy::result_large_err)]
fn revoke(svm: &mut LiteSVM, admin: &Keypair, org: Pubkey, member: Pubkey) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::RevokeMembership {}.data(),
        organization::accounts::RevokeMembership {
            authority: admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &admin.pubkey()),
            member,
            membership: membership_pda(&org, &member),
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(svm, admin, &[ix])
}

#[allow(clippy::result_large_err)]
fn update_role(
    svm: &mut LiteSVM,
    admin: &Keypair,
    org: Pubkey,
    member: Pubkey,
    new_role: Role,
) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::UpdateRole { new_role }.data(),
        organization::accounts::UpdateRole {
            authority: admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &admin.pubkey()),
            member,
            membership: membership_pda(&org, &member),
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(svm, admin, &[ix])
}

fn membership_state(svm: &LiteSVM, org: &Pubkey, member: &Pubkey) -> Membership {
    let account = svm.get_account(&membership_pda(org, member)).unwrap();
    let mut data: &[u8] = &account.data;
    Membership::try_deserialize(&mut data).unwrap()
}

/// Org owned by `authority` with a second Admin and a Submitter.
fn org_with_members(svm: &mut LiteSVM) -> (Keypair, Keypair, Pubkey, Pubkey) {
    let authority = funded(svm);
    let second_admin = funded(svm);
    let submitter = Pubkey::new_unique();
    assert_ok(&try_init(svm, &authority, "guarded", "Guarded"));
    let org = org_address("guarded");
    grant(svm, &authority, org, second_admin.pubkey(), Role::Admin);
    grant(svm, &authority, org, submitter, Role::Submitter);
    (authority, second_admin, org, submitter)
}

#[test]
fn slug_of_33_bytes_is_refused_without_panicking() {
    let mut svm = load_svm();
    let authority = funded(&mut svm);
    let slug = "a".repeat(33);
    assert_error_code(&try_init(&mut svm, &authority, &slug, "Name"), SLUG_INVALID);
}

#[test]
fn slug_outside_the_allowed_alphabet_or_empty_is_refused() {
    let mut svm = load_svm();
    let authority = funded(&mut svm);
    for slug in ["", "Upper", "under_score", "dot.ted", "space d", "café"] {
        assert_error_code(&try_init(&mut svm, &authority, slug, "Name"), SLUG_INVALID);
    }
}

#[test]
fn slug_at_the_bounds_is_accepted() {
    let mut svm = load_svm();
    let authority = funded(&mut svm);
    assert_ok(&try_init(&mut svm, &authority, &"z".repeat(32), "Name"));
    assert_ok(&try_init(&mut svm, &authority, "a", "Name"));
    assert_ok(&try_init(&mut svm, &authority, "spiral-hq-2026", "Name"));
}

#[test]
fn name_out_of_bounds_is_refused() {
    let mut svm = load_svm();
    let authority = funded(&mut svm);
    assert_error_code(&try_init(&mut svm, &authority, "n1", ""), NAME_EMPTY);
    assert_error_code(
        &try_init(&mut svm, &authority, "n2", &"x".repeat(97)),
        NAME_TOO_LONG,
    );
    assert!(svm.get_account(&org_address("n2")).is_none());
    assert_ok(&try_init(&mut svm, &authority, "n3", &"x".repeat(96)));
}

#[test]
fn authority_cannot_be_revoked() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    let target = authority.pubkey();
    assert_error_code(
        &revoke(&mut svm, &authority, org, target),
        AUTHORITY_PROTECTED,
    );
    assert_error_code(
        &revoke(&mut svm, &second_admin, org, target),
        AUTHORITY_PROTECTED,
    );
    assert_eq!(
        membership_state(&svm, &org, &target).status,
        MembershipStatus::Active
    );
}

#[test]
fn authority_cannot_be_demoted() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    let target = authority.pubkey();
    assert_error_code(
        &update_role(&mut svm, &authority, org, target, Role::Reader),
        AUTHORITY_PROTECTED,
    );
    assert_error_code(
        &update_role(&mut svm, &second_admin, org, target, Role::Submitter),
        AUTHORITY_PROTECTED,
    );
    assert_eq!(membership_state(&svm, &org, &target).role, Role::Admin);
}

#[test]
fn other_admins_can_still_be_demoted_and_revoked() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    let target = second_admin.pubkey();
    assert_ok(&update_role(
        &mut svm,
        &authority,
        org,
        target,
        Role::Submitter,
    ));
    assert_ok(&revoke(&mut svm, &authority, org, target));
}

#[test]
fn double_revoke_is_refused_and_keeps_revoked_at() {
    let mut svm = load_svm();
    let (authority, _, org, submitter) = org_with_members(&mut svm);
    assert_ok(&revoke(&mut svm, &authority, org, submitter));
    let first = membership_state(&svm, &org, &submitter).revoked_at;

    svm.set_sysvar(&Clock {
        unix_timestamp: 1_700_000_500,
        ..Default::default()
    });
    assert_error_code(
        &revoke(&mut svm, &authority, org, submitter),
        MEMBERSHIP_NOT_ACTIVE,
    );
    assert_eq!(membership_state(&svm, &org, &submitter).revoked_at, first);
}

#[allow(clippy::result_large_err)]
fn transfer(
    svm: &mut LiteSVM,
    authority: &Keypair,
    org: Pubkey,
    new_authority: Pubkey,
) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::TransferAuthority {}.data(),
        organization::accounts::TransferAuthority {
            authority: authority.pubkey(),
            organization: org,
            new_authority,
            new_authority_membership: membership_pda(&org, &new_authority),
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(svm, authority, &[ix])
}

fn org_authority(svm: &LiteSVM, org: &Pubkey) -> Pubkey {
    let account = svm.get_account(org).unwrap();
    let mut data: &[u8] = &account.data;
    organization::Organization::try_deserialize(&mut data)
        .unwrap()
        .authority
}

#[test]
fn transfer_to_a_non_member_is_refused() {
    let mut svm = load_svm();
    let (authority, _, org, _) = org_with_members(&mut svm);
    assert_error_code(
        &transfer(&mut svm, &authority, org, Pubkey::new_unique()),
        3012,
    );
    assert_eq!(org_authority(&svm, &org), authority.pubkey());
}

#[test]
fn transfer_to_a_revoked_member_is_refused() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    assert_ok(&revoke(&mut svm, &authority, org, second_admin.pubkey()));
    assert_error_code(
        &transfer(&mut svm, &authority, org, second_admin.pubkey()),
        6001,
    );
    assert_eq!(org_authority(&svm, &org), authority.pubkey());
}

#[test]
fn transfer_to_a_non_admin_is_refused() {
    let mut svm = load_svm();
    let (authority, _, org, submitter) = org_with_members(&mut svm);
    assert_error_code(&transfer(&mut svm, &authority, org, submitter), 6002);
    let reader = Pubkey::new_unique();
    grant(&mut svm, &authority, org, reader, Role::Reader);
    assert_error_code(&transfer(&mut svm, &authority, org, reader), 6002);
    assert_eq!(org_authority(&svm, &org), authority.pubkey());
}

#[test]
fn transfer_to_an_active_admin_moves_the_protection() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    assert_ok(&transfer(&mut svm, &authority, org, second_admin.pubkey()));
    assert_eq!(org_authority(&svm, &org), second_admin.pubkey());

    assert_error_code(
        &revoke(&mut svm, &authority, org, second_admin.pubkey()),
        AUTHORITY_PROTECTED,
    );
    assert_error_code(
        &update_role(
            &mut svm,
            &authority,
            org,
            second_admin.pubkey(),
            Role::Reader,
        ),
        AUTHORITY_PROTECTED,
    );
    assert_ok(&revoke(&mut svm, &second_admin, org, authority.pubkey()));
    assert_eq!(
        membership_state(&svm, &org, &authority.pubkey()).status,
        MembershipStatus::Revoked
    );
}

fn member_count(svm: &LiteSVM, org: &Pubkey) -> u32 {
    let account = svm.get_account(org).unwrap();
    let mut data: &[u8] = &account.data;
    organization::Organization::try_deserialize(&mut data)
        .unwrap()
        .member_count
}

#[test]
fn grant_at_u32_max_member_count_is_an_error_not_a_panic() {
    let mut svm = load_svm();
    let (authority, _, org, _) = org_with_members(&mut svm);
    let mut account = svm.get_account(&org).unwrap();
    let mut state = {
        let mut data: &[u8] = &account.data;
        organization::Organization::try_deserialize(&mut data).unwrap()
    };
    state.member_count = u32::MAX;
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&state, &mut data).unwrap();
    account.data[..data.len()].copy_from_slice(&data);
    svm.set_account(org, account).unwrap();

    let res = try_grant(
        &mut svm,
        &authority,
        org,
        Pubkey::new_unique(),
        Role::Reader,
    );
    assert_error_code(&res, MEMBER_COUNT_OVERFLOW);
}

#[test]
fn member_count_counts_every_grant_and_ignores_revokes() {
    let mut svm = load_svm();
    let (authority, _, org, submitter) = org_with_members(&mut svm);
    assert_eq!(member_count(&svm, &org), 3);
    assert_ok(&revoke(&mut svm, &authority, org, submitter));
    assert_eq!(member_count(&svm, &org), 3);
}

fn events<E: AnchorDeserialize + Discriminator>(res: &TransactionResult) -> Vec<E> {
    let meta = res.as_ref().expect("transaction failed");
    meta.inner_instructions
        .iter()
        .flatten()
        .filter_map(|inner| {
            let data = &inner.instruction.data;
            let rest = data.strip_prefix(anchor_lang::event::EVENT_IX_TAG_LE)?;
            let mut body = rest.strip_prefix(E::DISCRIMINATOR)?;
            Some(E::deserialize(&mut body).unwrap())
        })
        .collect()
}

#[test]
fn grant_update_role_and_transfer_emit_events() {
    let mut svm = load_svm();
    let authority = funded(&mut svm);
    let other_admin = funded(&mut svm);
    assert_ok(&try_init(&mut svm, &authority, "evented", "Evented"));
    let org = org_address("evented");
    let member = other_admin.pubkey();

    let res = try_grant(&mut svm, &authority, org, member, Role::Submitter);
    let granted = events::<organization::MembershipGranted>(&res);
    assert_eq!(granted.len(), 1);
    assert_eq!(granted[0].organization, org);
    assert_eq!(granted[0].member, member);
    assert_eq!(granted[0].role, Role::Submitter);
    assert_eq!(granted[0].granted_by, authority.pubkey());
    assert_eq!(granted[0].granted_at, 1_700_000_000);

    let res = update_role(&mut svm, &authority, org, member, Role::Admin);
    let updated = events::<organization::MembershipRoleUpdated>(&res);
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].organization, org);
    assert_eq!(updated[0].member, member);
    assert_eq!(updated[0].authority, authority.pubkey());
    assert_eq!(updated[0].old_role, Role::Submitter);
    assert_eq!(updated[0].new_role, Role::Admin);

    let res = transfer(&mut svm, &authority, org, member);
    let moved = events::<organization::AuthorityTransferred>(&res);
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].organization, org);
    assert_eq!(moved[0].old_authority, authority.pubkey());
    assert_eq!(moved[0].new_authority, member);
}

#[allow(clippy::result_large_err)]
fn assert_can_submit(svm: &mut LiteSVM, member: &Keypair, org: Pubkey) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::AssertCanSubmit {}.data(),
        organization::accounts::AssertCanSubmit {
            organization: org,
            membership: membership_pda(&org, &member.pubkey()),
            member: member.pubkey(),
        }
        .to_account_metas(None),
    );
    send(svm, member, &[ix])
}

fn assert_admin_actions_refused(
    svm: &mut LiteSVM,
    signer: &Keypair,
    org: Pubkey,
    target: Pubkey,
    expected: u32,
) {
    assert_error_code(
        &try_grant(svm, signer, org, Pubkey::new_unique(), Role::Reader),
        expected,
    );
    assert_error_code(
        &update_role(svm, signer, org, target, Role::Reader),
        expected,
    );
    assert_error_code(&revoke(svm, signer, org, target), expected);
}

#[test]
fn non_admin_cannot_grant_update_or_revoke() {
    let mut svm = load_svm();
    let (authority, _, org, target) = org_with_members(&mut svm);
    let submitter = funded(&mut svm);
    let reader = funded(&mut svm);
    grant(
        &mut svm,
        &authority,
        org,
        submitter.pubkey(),
        Role::Submitter,
    );
    grant(&mut svm, &authority, org, reader.pubkey(), Role::Reader);
    let count = member_count(&svm, &org);

    assert_admin_actions_refused(&mut svm, &submitter, org, target, INSUFFICIENT_ROLE);
    assert_admin_actions_refused(&mut svm, &reader, org, target, INSUFFICIENT_ROLE);

    assert_eq!(member_count(&svm, &org), count);
    let state = membership_state(&svm, &org, &target);
    assert_eq!(state.role, Role::Submitter);
    assert_eq!(state.status, MembershipStatus::Active);
}

#[test]
fn revoked_admin_cannot_grant_update_or_revoke() {
    let mut svm = load_svm();
    let (authority, second_admin, org, target) = org_with_members(&mut svm);
    assert_ok(&revoke(&mut svm, &authority, org, second_admin.pubkey()));
    let count = member_count(&svm, &org);

    assert_admin_actions_refused(&mut svm, &second_admin, org, target, MEMBERSHIP_REVOKED);

    assert_eq!(member_count(&svm, &org), count);
    let state = membership_state(&svm, &org, &target);
    assert_eq!(state.role, Role::Submitter);
    assert_eq!(state.status, MembershipStatus::Active);
}

#[test]
fn transfer_signed_by_a_non_authority_is_refused() {
    let mut svm = load_svm();
    let (authority, second_admin, org, _) = org_with_members(&mut svm);
    assert_error_code(
        &transfer(&mut svm, &second_admin, org, second_admin.pubkey()),
        2001,
    );
    assert_eq!(org_authority(&svm, &org), authority.pubkey());
}

#[test]
fn assert_can_submit_checks_role_and_status() {
    let mut svm = load_svm();
    let (authority, _, org, _) = org_with_members(&mut svm);
    let submitter = funded(&mut svm);
    let reader = funded(&mut svm);
    grant(
        &mut svm,
        &authority,
        org,
        submitter.pubkey(),
        Role::Submitter,
    );
    grant(&mut svm, &authority, org, reader.pubkey(), Role::Reader);

    assert_ok(&assert_can_submit(&mut svm, &authority, org));
    assert_ok(&assert_can_submit(&mut svm, &submitter, org));
    assert_error_code(
        &assert_can_submit(&mut svm, &reader, org),
        INSUFFICIENT_ROLE,
    );

    assert_ok(&revoke(&mut svm, &authority, org, submitter.pubkey()));
    assert_error_code(
        &assert_can_submit(&mut svm, &submitter, org),
        MEMBERSHIP_REVOKED,
    );
}
