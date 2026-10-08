#[path = "../../../test-support/program_bytes.rs"]
mod program_bytes;

use program_bytes::program_bytes;

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, AccountSerialize, AnchorDeserialize, Discriminator, InstructionData,
        ToAccountMetas,
    },
    film::AccessPolicy,
    litesvm::{types::TransactionResult, LiteSVM},
    organization::{Membership, MembershipStatus, Role},
    solana_account::Account,
    solana_instruction::error::InstructionError,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_error::TransactionError,
};

const ED25519_PREFIX: [u8; 6] = [0x00, 0x24, 0x08, 0x01, 0x12, 0x20];
const MEMBERSHIP_REVOKED: u32 = 6001;
const INSUFFICIENT_ROLE: u32 = 6002;
const ORG_MISMATCH: u32 = 6003;
const INVALID_REPLICA_INDEX: u32 = 6008;
const CONSTRAINT_SEEDS: u32 = 2006;
const ACCOUNT_OWNED_BY_WRONG_PROGRAM: u32 = 3007;

#[derive(Clone, Copy, Debug)]
enum Op {
    Declare,
    Deactivate,
    Policy,
}

const OPS: [Op; 3] = [Op::Declare, Op::Deactivate, Op::Policy];

struct Ctx {
    svm: LiteSVM,
    admin: Keypair,
    submitter: Keypair,
    other_submitter: Keypair,
    outsider: Keypair,
    org: Pubkey,
    other_org: Pubkey,
    film: Pubkey,
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

fn peer_id(seed: u8) -> [u8; 38] {
    let key = Keypair::new_from_array([seed; 32]).pubkey().to_bytes();
    let mut p = [0u8; 38];
    p[..6].copy_from_slice(&ED25519_PREFIX);
    p[6..].copy_from_slice(&key);
    p
}

fn film_event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0
}

fn init_org(svm: &mut LiteSVM, authority: &Keypair, slug: &str) -> Pubkey {
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    let (org, _) =
        Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.to_string(),
            name: slug.to_string(),
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
    assert_ok(&send(svm, authority, &[ix]));
    org
}

fn grant(svm: &mut LiteSVM, admin: &Keypair, org: Pubkey, member: &Keypair, role: Role) {
    svm.airdrop(&member.pubkey(), 10_000_000_000).unwrap();
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::GrantMembership { role }.data(),
        organization::accounts::GrantMembership {
            authority: admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &admin.pubkey()),
            member: member.pubkey(),
            membership: membership_pda(&org, &member.pubkey()),
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
    assert_ok(&send(svm, admin, &[ix]));
}

fn setup() -> Ctx {
    let mut svm = LiteSVM::new();
    svm.add_program(organization::id(), &program_bytes("organization"))
        .unwrap();
    svm.add_program(film::id(), &program_bytes("film")).unwrap();

    let admin = Keypair::new();
    let submitter = Keypair::new();
    let other_submitter = Keypair::new();
    let outsider = Keypair::new();

    let org = init_org(&mut svm, &admin, "authz");
    grant(&mut svm, &admin, org, &submitter, Role::Submitter);
    grant(&mut svm, &admin, org, &other_submitter, Role::Submitter);
    let other_org = init_org(&mut svm, &outsider, "elsewhere");

    let mut cid = [0u8; 64];
    cid[..4].copy_from_slice(&[0x01, 0x70, 0x12, 0x20]);
    cid[4] = 3;
    let digest = solana_sha256_hasher::hash(&cid[..36]);
    let (film_pda, _) = Pubkey::find_program_address(
        &[b"film", org.as_ref(), &digest.to_bytes()[..32]],
        &film::id(),
    );
    let register_ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::RegisterFilm {
            cid,
            cid_len: 36,
            byte_size: 1,
            chunking_profile: 1,
            title: "T".to_string(),
        }
        .data(),
        film::accounts::RegisterFilm {
            submitter: submitter.pubkey(),
            organization: org,
            membership: membership_pda(&org, &submitter.pubkey()),
            film: film_pda,
            organization_program: organization::id(),
            system_program: system_program::ID,
            event_authority: film_event_authority(),
            program: film::id(),
        }
        .to_account_metas(None),
    );
    assert_ok(&send(&mut svm, &submitter, &[register_ix]));

    let mut ctx = Ctx {
        svm,
        admin,
        submitter,
        other_submitter,
        outsider,
        org,
        other_org,
        film: film_pda,
    };
    let (submitter, org) = (ctx.submitter.insecure_clone(), ctx.org);
    let membership = membership_pda(&org, &submitter.pubkey());
    assert_ok(&run_with(
        &mut ctx,
        Op::Declare,
        &submitter,
        org,
        membership,
        0,
    ));
    ctx
}

fn op_ix(
    ctx: &Ctx,
    op: Op,
    authority: Pubkey,
    organization: Pubkey,
    membership: Pubkey,
    index: u8,
) -> Instruction {
    let film = ctx.film;
    let organization_program = organization::id();
    let event_authority = film_event_authority();
    let program = film::id();
    match op {
        Op::Declare => Instruction::new_with_bytes(
            film::id(),
            &film::instruction::DeclareReplica {
                peer_id: peer_id(index.wrapping_add(1)),
                peer_id_len: 38,
            }
            .data(),
            film::accounts::DeclareReplica {
                authority,
                organization,
                membership,
                organization_program,
                film,
                event_authority,
                program,
            }
            .to_account_metas(None),
        ),
        Op::Deactivate => Instruction::new_with_bytes(
            film::id(),
            &film::instruction::DeactivateReplica { index }.data(),
            film::accounts::DeactivateReplica {
                authority,
                organization,
                membership,
                organization_program,
                film,
                event_authority,
                program,
            }
            .to_account_metas(None),
        ),
        Op::Policy => Instruction::new_with_bytes(
            film::id(),
            &film::instruction::UpdateAccessPolicy {
                new_policy: AccessPolicy::OrganizationOnly,
            }
            .data(),
            film::accounts::UpdateAccessPolicy {
                authority,
                organization,
                membership,
                organization_program,
                film,
                event_authority,
                program,
            }
            .to_account_metas(None),
        ),
    }
}

#[allow(clippy::result_large_err)]
fn run_with(
    ctx: &mut Ctx,
    op: Op,
    signer: &Keypair,
    organization: Pubkey,
    membership: Pubkey,
    index: u8,
) -> TransactionResult {
    let ix = op_ix(ctx, op, signer.pubkey(), organization, membership, index);
    send(&mut ctx.svm, signer, &[ix])
}

/// Runs `op` as `signer` against the film's organization with the signer's own membership PDA.
/// Declares use a fresh PeerID; deactivations target the slot declared in `setup`.
#[allow(clippy::result_large_err)]
fn run(ctx: &mut Ctx, op: Op, signer: &Keypair) -> TransactionResult {
    let org = ctx.org;
    let membership = membership_pda(&org, &signer.pubkey());
    let index = match op {
        Op::Declare => 1,
        _ => 0,
    };
    run_with(ctx, op, signer, org, membership, index)
}

fn film_state(ctx: &Ctx) -> film::Film {
    let account = ctx.svm.get_account(&ctx.film).unwrap();
    let mut data: &[u8] = &account.data;
    film::Film::try_deserialize(&mut data).unwrap()
}

fn assert_effect(ctx: &Ctx, op: Op) {
    let state = film_state(ctx);
    match op {
        Op::Declare => {
            assert_eq!(state.replica_count, 2);
            assert!(state.replicas[1].active);
        }
        Op::Deactivate => assert!(!state.replicas[0].active),
        Op::Policy => assert_eq!(state.access_policy, AccessPolicy::OrganizationOnly),
    }
}

fn assert_untouched(ctx: &Ctx) {
    let state = film_state(ctx);
    assert_eq!(state.replica_count, 1);
    assert!(state.replicas[0].active);
    assert_eq!(state.access_policy, AccessPolicy::Public);
}

fn revoke(ctx: &mut Ctx, member: Pubkey) {
    let org = ctx.org;
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::RevokeMembership {}.data(),
        organization::accounts::RevokeMembership {
            authority: ctx.admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &ctx.admin.pubkey()),
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
    let admin = ctx.admin.insecure_clone();
    assert_ok(&send(&mut ctx.svm, &admin, &[ix]));
}

fn update_role(ctx: &mut Ctx, member: Pubkey, new_role: Role) {
    let org = ctx.org;
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::UpdateRole { new_role }.data(),
        organization::accounts::UpdateRole {
            authority: ctx.admin.pubkey(),
            organization: org,
            admin_membership: membership_pda(&org, &ctx.admin.pubkey()),
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
    let admin = ctx.admin.insecure_clone();
    assert_ok(&send(&mut ctx.svm, &admin, &[ix]));
}

#[test]
fn active_submitter_can_manage() {
    for op in OPS {
        let mut ctx = setup();
        let s = ctx.submitter.insecure_clone();
        assert_ok(&run(&mut ctx, op, &s));
        assert_effect(&ctx, op);
    }
}

#[test]
fn revoked_submitter_is_refused() {
    for op in OPS {
        let mut ctx = setup();
        let member = ctx.submitter.pubkey();
        revoke(&mut ctx, member);
        let s = ctx.submitter.insecure_clone();
        assert_error_code(&run(&mut ctx, op, &s), MEMBERSHIP_REVOKED);
        assert_untouched(&ctx);
    }
}

#[test]
fn submitter_demoted_to_reader_is_refused() {
    for op in OPS {
        let mut ctx = setup();
        let member = ctx.submitter.pubkey();
        update_role(&mut ctx, member, Role::Reader);
        let s = ctx.submitter.insecure_clone();
        assert_error_code(&run(&mut ctx, op, &s), INSUFFICIENT_ROLE);
        assert_untouched(&ctx);
    }
}

#[test]
fn another_submitter_of_the_org_is_refused() {
    for op in OPS {
        let mut ctx = setup();
        let s = ctx.other_submitter.insecure_clone();
        assert_error_code(&run(&mut ctx, op, &s), INSUFFICIENT_ROLE);
        assert_untouched(&ctx);
    }
}

#[test]
fn org_admin_who_is_not_the_submitter_can_manage() {
    for op in OPS {
        let mut ctx = setup();
        let a = ctx.admin.insecure_clone();
        assert_ok(&run(&mut ctx, op, &a));
        assert_effect(&ctx, op);
    }
}

#[test]
fn member_of_another_org_is_refused() {
    for op in OPS {
        let mut ctx = setup();
        let o = ctx.outsider.insecure_clone();
        let other_org = ctx.other_org;
        let membership = membership_pda(&other_org, &o.pubkey());
        let index = matches!(op, Op::Declare) as u8;
        assert_error_code(
            &run_with(&mut ctx, op, &o, other_org, membership, index),
            ORG_MISMATCH,
        );
        assert_untouched(&ctx);
    }
}

#[test]
fn other_org_membership_under_the_films_org_fails_seeds() {
    for op in OPS {
        let mut ctx = setup();
        let o = ctx.outsider.insecure_clone();
        let org = ctx.org;
        let membership = membership_pda(&ctx.other_org, &o.pubkey());
        let index = matches!(op, Op::Declare) as u8;
        assert_error_code(
            &run_with(&mut ctx, op, &o, org, membership, index),
            CONSTRAINT_SEEDS,
        );
        assert_untouched(&ctx);
    }
}

#[test]
fn forged_membership_is_refused_by_seeds() {
    for op in OPS {
        let mut ctx = setup();
        let s = ctx.submitter.insecure_clone();
        let forged = Pubkey::new_unique();
        let real = membership_pda(&ctx.org, &s.pubkey());
        let mut fake = Membership {
            organization: ctx.org,
            member: s.pubkey(),
            role: Role::Admin,
            status: MembershipStatus::Active,
            granted_by: ctx.admin.pubkey(),
            granted_at: 0,
            revoked_at: 0,
            bump: 0,
        };
        let real_account = ctx.svm.get_account(&real).unwrap();
        fake.bump = {
            let mut d: &[u8] = &real_account.data;
            Membership::try_deserialize(&mut d).unwrap().bump
        };
        let mut data = Vec::new();
        fake.try_serialize(&mut data).unwrap();
        ctx.svm
            .set_account(
                forged,
                Account {
                    lamports: real_account.lamports,
                    data,
                    owner: organization::id(),
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        let org = ctx.org;
        let index = matches!(op, Op::Declare) as u8;
        assert_error_code(
            &run_with(&mut ctx, op, &s, org, forged, index),
            CONSTRAINT_SEEDS,
        );
        assert_untouched(&ctx);
    }
}

#[test]
fn membership_owned_by_another_program_is_refused() {
    for op in OPS {
        let mut ctx = setup();
        let s = ctx.submitter.insecure_clone();
        let real = ctx
            .svm
            .get_account(&membership_pda(&ctx.org, &s.pubkey()))
            .unwrap();
        let forged = Pubkey::new_unique();
        ctx.svm
            .set_account(
                forged,
                Account {
                    owner: film::id(),
                    ..real
                },
            )
            .unwrap();
        let org = ctx.org;
        let index = matches!(op, Op::Declare) as u8;
        assert_error_code(
            &run_with(&mut ctx, op, &s, org, forged, index),
            ACCOUNT_OWNED_BY_WRONG_PROGRAM,
        );
        assert_untouched(&ctx);
    }
}

#[test]
fn another_members_membership_is_refused_by_seeds() {
    for op in OPS {
        let mut ctx = setup();
        let s = ctx.other_submitter.insecure_clone();
        let org = ctx.org;
        let admin_membership = membership_pda(&org, &ctx.admin.pubkey());
        let index = matches!(op, Op::Declare) as u8;
        assert_error_code(
            &run_with(&mut ctx, op, &s, org, admin_membership, index),
            CONSTRAINT_SEEDS,
        );
        assert_untouched(&ctx);
    }
}

#[test]
fn deactivate_out_of_range_index() {
    let mut ctx = setup();
    let s = ctx.submitter.insecure_clone();
    let org = ctx.org;
    let membership = membership_pda(&org, &s.pubkey());
    assert_error_code(
        &run_with(&mut ctx, Op::Deactivate, &s, org, membership, 1),
        INVALID_REPLICA_INDEX,
    );
    assert_untouched(&ctx);
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
fn management_emits_events() {
    let mut ctx = setup();
    let s = ctx.submitter.insecure_clone();

    let res = run(&mut ctx, Op::Declare, &s);
    let declared = events::<film::ReplicaDeclared>(&res);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].film, ctx.film);
    assert_eq!(declared[0].authority, s.pubkey());
    assert_eq!(declared[0].peer_id, peer_id(2));
    assert_eq!(declared[0].peer_id_len, 38);
    assert_eq!(declared[0].slot, 1);
    assert!(!declared[0].reactivated);
    assert_eq!(declared[0].replaced_peer_id, None);

    let res = run(&mut ctx, Op::Deactivate, &s);
    let deactivated = events::<film::ReplicaDeactivated>(&res);
    assert_eq!(deactivated.len(), 1);
    assert_eq!(deactivated[0].film, ctx.film);
    assert_eq!(deactivated[0].authority, s.pubkey());
    assert_eq!(deactivated[0].slot, 0);
    assert_eq!(deactivated[0].peer_id, peer_id(1));
    assert!(deactivated[0].was_active);

    let res = run(&mut ctx, Op::Policy, &s);
    let updated = events::<film::AccessPolicyUpdated>(&res);
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].film, ctx.film);
    assert_eq!(updated[0].authority, s.pubkey());
    assert_eq!(updated[0].policy, AccessPolicy::OrganizationOnly);
}

#[test]
fn repeated_deactivation_reports_was_active_false() {
    let mut ctx = setup();
    let s = ctx.submitter.insecure_clone();
    assert_ok(&run(&mut ctx, Op::Deactivate, &s));
    let res = run(&mut ctx, Op::Deactivate, &s);
    let deactivated = events::<film::ReplicaDeactivated>(&res);
    assert_eq!(deactivated.len(), 1);
    assert!(!deactivated[0].was_active);
    assert!(!film_state(&ctx).replicas[0].active);
}

#[test]
fn admin_intervention_is_attributed_in_events() {
    for op in OPS {
        let mut ctx = setup();
        let a = ctx.admin.insecure_clone();
        let res = run(&mut ctx, op, &a);
        assert_ok(&res);
        let authority = match op {
            Op::Declare => events::<film::ReplicaDeclared>(&res)[0].authority,
            Op::Deactivate => events::<film::ReplicaDeactivated>(&res)[0].authority,
            Op::Policy => events::<film::AccessPolicyUpdated>(&res)[0].authority,
        };
        assert_eq!(authority, a.pubkey());
        assert_ne!(authority, ctx.submitter.pubkey());
    }
}
