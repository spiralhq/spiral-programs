#[path = "../../../test-support/program_bytes.rs"]
mod program_bytes;

use program_bytes::program_bytes;

use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, AccountSerialize, AnchorDeserialize, Discriminator, InstructionData,
        Space, ToAccountMetas,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_instruction::error::InstructionError,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_error::TransactionError,
};

const ED25519_PREFIX: [u8; 6] = [0x00, 0x24, 0x08, 0x01, 0x12, 0x20];

struct Ctx {
    svm: LiteSVM,
    submitter: Keypair,
    org: Pubkey,
    membership: Pubkey,
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

fn peer_id(seed: u8) -> [u8; 38] {
    let key = Keypair::new_from_array([seed; 32]).pubkey().to_bytes();
    let mut p = [0u8; 38];
    p[..6].copy_from_slice(&ED25519_PREFIX);
    p[6..].copy_from_slice(&key);
    p
}

fn setup() -> Ctx {
    let mut svm = LiteSVM::new();
    svm.add_program(organization::id(), &program_bytes("organization"))
        .unwrap();
    svm.add_program(film::id(), &program_bytes("film")).unwrap();

    let submitter = Keypair::new();
    svm.airdrop(&submitter.pubkey(), 10_000_000_000).unwrap();

    let slug = "replicas";
    let (org, _) =
        Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let (membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), submitter.pubkey().as_ref()],
        &organization::id(),
    );
    let init_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.to_string(),
            name: slug.to_string(),
        }
        .data(),
        organization::accounts::InitializeOrganization {
            authority: submitter.pubkey(),
            organization: org,
            membership,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    assert_ok(&send(&mut svm, &submitter, &[init_ix]));

    let mut cid = [0u8; 64];
    cid[..4].copy_from_slice(&[0x01, 0x70, 0x12, 0x20]);
    cid[4] = 7;
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
            membership,
            film: film_pda,
            organization_program: organization::id(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    assert_ok(&send(&mut svm, &submitter, &[register_ix]));

    Ctx {
        svm,
        submitter,
        org,
        membership,
        film: film_pda,
    }
}

#[allow(clippy::result_large_err)]
fn declare(ctx: &mut Ctx, peer_id: [u8; 38], peer_id_len: u8) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::DeclareReplica {
            peer_id,
            peer_id_len,
        }
        .data(),
        film::accounts::DeclareReplica {
            authority: ctx.submitter.pubkey(),
            organization: ctx.org,
            membership: ctx.membership,
            organization_program: organization::id(),
            film: ctx.film,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    send(&mut ctx.svm, &ctx.submitter, &[ix])
}

#[allow(clippy::result_large_err)]
fn deactivate(ctx: &mut Ctx, index: u8) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::DeactivateReplica { index }.data(),
        film::accounts::DeactivateReplica {
            authority: ctx.submitter.pubkey(),
            organization: ctx.org,
            membership: ctx.membership,
            organization_program: organization::id(),
            film: ctx.film,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    send(&mut ctx.svm, &ctx.submitter, &[ix])
}

fn film_state(ctx: &Ctx) -> film::Film {
    let account = ctx.svm.get_account(&ctx.film).unwrap();
    let mut data: &[u8] = &account.data;
    film::Film::try_deserialize(&mut data).unwrap()
}

fn advance_clock(ctx: &mut Ctx, seconds: i64) {
    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += seconds;
    ctx.svm.set_sysvar::<Clock>(&clock);
}

#[test]
fn valid_declare_appends_active_slot() {
    let mut ctx = setup();
    let p = peer_id(1);
    assert_ok(&declare(&mut ctx, p, 38));
    let state = film_state(&ctx);
    assert_eq!(state.replica_count, 1);
    assert_eq!(state.replicas[0].peer_id, p);
    assert_eq!(state.replicas[0].peer_id_len, 38);
    assert!(state.replicas[0].active);
}

#[test]
fn length_37_is_invalid_peer_id() {
    let mut ctx = setup();
    assert_error_code(&declare(&mut ctx, peer_id(1), 37), 6009);
    assert_eq!(film_state(&ctx).replica_count, 0);
}

#[test]
fn length_39_is_invalid_peer_id() {
    let mut ctx = setup();
    assert_error_code(&declare(&mut ctx, peer_id(1), 39), 6009);
    assert_eq!(film_state(&ctx).replica_count, 0);
}

#[test]
fn wrong_prefix_is_invalid_peer_id() {
    let mut ctx = setup();
    let mut p = peer_id(1);
    p[2] = 0x09;
    assert_error_code(&declare(&mut ctx, p, 38), 6009);
    assert_eq!(film_state(&ctx).replica_count, 0);
}

#[test]
fn duplicate_active_peer_id_is_refused() {
    let mut ctx = setup();
    let p = peer_id(1);
    assert_ok(&declare(&mut ctx, p, 38));
    assert_error_code(&declare(&mut ctx, p, 38), 6010);
    assert_eq!(film_state(&ctx).replica_count, 1);
}

#[test]
fn redeclare_after_deactivate_reactivates_same_slot() {
    let mut ctx = setup();
    assert_ok(&declare(&mut ctx, peer_id(1), 38));
    assert_ok(&declare(&mut ctx, peer_id(2), 38));
    let before = film_state(&ctx).replicas[0].declared_at;

    assert_ok(&deactivate(&mut ctx, 0));
    assert!(!film_state(&ctx).replicas[0].active);

    advance_clock(&mut ctx, 100);
    assert_ok(&declare(&mut ctx, peer_id(1), 38));
    let state = film_state(&ctx);
    assert_eq!(state.replica_count, 2);
    assert!(state.replicas[0].active);
    assert_eq!(state.replicas[0].peer_id, peer_id(1));
    assert!(state.replicas[0].declared_at > before);
    assert_eq!(state.replicas[2].peer_id_len, 0);
}

#[test]
fn ninth_distinct_peer_exceeds_capacity() {
    let mut ctx = setup();
    for seed in 1..=8 {
        assert_ok(&declare(&mut ctx, peer_id(seed), 38));
    }
    assert_error_code(&declare(&mut ctx, peer_id(9), 38), 6007);
    assert_eq!(film_state(&ctx).replica_count, 8);
}

#[test]
fn ninth_distinct_peer_reuses_first_inactive_slot() {
    let mut ctx = setup();
    for seed in 1..=8 {
        assert_ok(&declare(&mut ctx, peer_id(seed), 38));
    }
    assert_ok(&deactivate(&mut ctx, 5));
    assert_ok(&deactivate(&mut ctx, 3));
    advance_clock(&mut ctx, 100);
    assert_ok(&declare(&mut ctx, peer_id(9), 38));

    let state = film_state(&ctx);
    assert_eq!(state.replica_count, 8);
    assert_eq!(state.replicas[3].peer_id, peer_id(9));
    assert!(state.replicas[3].active);
    assert_eq!(state.replicas[5].peer_id, peer_id(6));
    assert!(!state.replicas[5].active);
}

#[test]
fn account_layout_is_unchanged() {
    let mut ctx = setup();
    assert_ok(&declare(&mut ctx, peer_id(1), 38));
    let account = ctx.svm.get_account(&ctx.film).unwrap();
    assert_eq!(film::Film::INIT_SPACE, 698);
    assert_eq!(account.data.len(), 8 + 698);
    let state = film_state(&ctx);
    assert_eq!(state.replicas.len(), 8);
    assert_eq!(state.replicas[0].peer_id_len, 38);
}

fn events<E: AnchorDeserialize + Discriminator>(res: &TransactionResult) -> Vec<E> {
    let meta = res.as_ref().expect("transaction failed");
    meta.inner_instructions
        .iter()
        .flatten()
        .filter_map(|inner| {
            let rest = inner
                .instruction
                .data
                .strip_prefix(anchor_lang::event::EVENT_IX_TAG_LE)?;
            let mut body = rest.strip_prefix(E::DISCRIMINATOR)?;
            Some(E::deserialize(&mut body).unwrap())
        })
        .collect()
}

#[test]
fn full_with_active_duplicate_is_duplicate_not_capacity() {
    let mut ctx = setup();
    for seed in 1..=8 {
        assert_ok(&declare(&mut ctx, peer_id(seed), 38));
    }
    assert_error_code(&declare(&mut ctx, peer_id(4), 38), 6010);
}

#[test]
fn new_peer_appends_before_reusing_inactive_slot() {
    let mut ctx = setup();
    assert_ok(&declare(&mut ctx, peer_id(1), 38));
    assert_ok(&declare(&mut ctx, peer_id(2), 38));
    assert_ok(&deactivate(&mut ctx, 0));
    assert_ok(&declare(&mut ctx, peer_id(3), 38));

    let state = film_state(&ctx);
    assert_eq!(state.replica_count, 3);
    assert_eq!(state.replicas[2].peer_id, peer_id(3));
    assert!(state.replicas[2].active);
    assert_eq!(state.replicas[0].peer_id, peer_id(1));
    assert!(!state.replicas[0].active);
}

#[test]
fn reused_slot_advances_declared_at_and_reports_replaced_peer() {
    let mut ctx = setup();
    for seed in 1..=8 {
        assert_ok(&declare(&mut ctx, peer_id(seed), 38));
    }
    let before = film_state(&ctx).replicas[3].declared_at;
    assert_ok(&deactivate(&mut ctx, 3));
    advance_clock(&mut ctx, 100);

    let res = declare(&mut ctx, peer_id(9), 38);
    assert_ok(&res);
    let state = film_state(&ctx);
    assert_eq!(state.replicas[3].peer_id, peer_id(9));
    assert!(state.replicas[3].declared_at > before);

    let declared = events::<film::ReplicaDeclared>(&res);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].slot, 3);
    assert!(!declared[0].reactivated);
    assert_eq!(declared[0].replaced_peer_id, Some(peer_id(4)));
}

#[test]
fn reactivation_is_reported_in_event() {
    let mut ctx = setup();
    assert_ok(&declare(&mut ctx, peer_id(1), 38));
    assert_ok(&deactivate(&mut ctx, 0));
    let res = declare(&mut ctx, peer_id(1), 38);
    assert_ok(&res);
    let declared = events::<film::ReplicaDeclared>(&res);
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].slot, 0);
    assert!(declared[0].reactivated);
    assert_eq!(declared[0].replaced_peer_id, None);
}

#[test]
fn legacy_inactive_and_active_duplicate_is_refused() {
    let mut ctx = setup();
    let p = peer_id(1);
    let mut state = film_state(&ctx);
    for (i, active) in [(0, false), (1, true)] {
        state.replicas[i] = film::ReplicaDeclaration {
            peer_id: p,
            peer_id_len: 38,
            declared_at: 1,
            active,
        };
    }
    state.replica_count = 2;
    let mut account = ctx.svm.get_account(&ctx.film).unwrap();
    let mut data = Vec::new();
    state.try_serialize(&mut data).unwrap();
    account.data[..data.len()].copy_from_slice(&data);
    ctx.svm.set_account(ctx.film, account).unwrap();

    assert_error_code(&declare(&mut ctx, p, 38), 6010);
    let state = film_state(&ctx);
    assert!(!state.replicas[0].active);
    assert!(state.replicas[1].active);
}
