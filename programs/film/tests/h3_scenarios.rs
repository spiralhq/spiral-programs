use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_account::Account,
    solana_instruction::error::InstructionError,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    solana_transaction_error::TransactionError,
};

fn load_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program(
        organization::id(),
        include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/organization.so")),
    )
    .unwrap();
    svm.add_program(
        film::id(),
        include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/film.so")),
    )
    .unwrap();
    svm
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ixs: &[Instruction], extra: &[&Keypair]) -> TransactionResult {
    svm.expire_blockhash();
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &bh);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx)
}

fn assert_ok(res: &TransactionResult) {
    assert!(res.is_ok(), "expected success, got: {:?}", res.as_ref().err());
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

fn init_org(svm: &mut LiteSVM, authority: &Keypair, slug: &str) -> Pubkey {
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    let (org, _) = Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let (membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );
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
            membership,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    assert_ok(&send(svm, authority, &[ix], &[]));
    org
}

fn grant(svm: &mut LiteSVM, authority: &Keypair, org: Pubkey, member: Pubkey, role: organization::Role) -> Pubkey {
    let (admin_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );
    let (membership, _) =
        Pubkey::find_program_address(&[b"membership", org.as_ref(), member.as_ref()], &organization::id());
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::GrantMembership { role }.data(),
        organization::accounts::GrantMembership {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member,
            membership,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    assert_ok(&send(svm, authority, &[ix], &[]));
    membership
}

fn register_ix(org: Pubkey, submitter: Pubkey, membership: Pubkey, seed: u8) -> (Instruction, Pubkey) {
    let cid: [u8; 64] = {
        let mut c = [0u8; 64];
        c[0] = 0x01;
        c[1] = 0x70;
        c[2] = 0x12;
        c[3] = 0x20;
        c[4] = seed;
        c
    };
    let (film_pda, _) = {
        let digest = solana_sha256_hasher::hash(&cid[..36]);
        Pubkey::find_program_address(&[b"film", org.as_ref(), &digest.to_bytes()[..32]], &film::id())
    };
    let event_authority = Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0;
    let ix = Instruction::new_with_bytes(
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
            submitter,
            organization: org,
            membership,
            film: film_pda,
            organization_program: organization::id(),
            system_program: system_program::ID,
            event_authority,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    (ix, film_pda)
}

#[test]
fn scenario_1_no_membership() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let outsider = Keypair::new();
    svm.airdrop(&outsider.pubkey(), 10_000_000_000).unwrap();
    let org = init_org(&mut svm, &authority, "h3-1");

    let (outsider_membership, _) =
        Pubkey::find_program_address(&[b"membership", org.as_ref(), outsider.pubkey().as_ref()], &organization::id());
    let (ix, _) = register_ix(org, outsider.pubkey(), outsider_membership, 1);
    let res = send(&mut svm, &outsider, &[ix], &[]);
    assert_error_code(&res, 3012);
}

#[test]
fn scenario_2_revoked_membership() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let member = Keypair::new();
    svm.airdrop(&member.pubkey(), 10_000_000_000).unwrap();
    let org = init_org(&mut svm, &authority, "h3-2");
    let membership = grant(&mut svm, &authority, org, member.pubkey(), organization::Role::Submitter);

    let (admin_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );
    let revoke_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::RevokeMembership {}.data(),
        organization::accounts::RevokeMembership {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member: member.pubkey(),
            membership,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &organization::id()).0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    assert_ok(&send(&mut svm, &authority, &[revoke_ix], &[]));

    let (ix, _) = register_ix(org, member.pubkey(), membership, 2);
    let res = send(&mut svm, &member, &[ix], &[]);
    assert_error_code(&res, 6001);
}

#[test]
fn scenario_3_insufficient_role() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let member = Keypair::new();
    svm.airdrop(&member.pubkey(), 10_000_000_000).unwrap();
    let org = init_org(&mut svm, &authority, "h3-3");
    let membership = grant(&mut svm, &authority, org, member.pubkey(), organization::Role::Reader);

    let (ix, _) = register_ix(org, member.pubkey(), membership, 3);
    let res = send(&mut svm, &member, &[ix], &[]);
    assert_error_code(&res, 6002);
}

#[test]
fn scenario_4_org_mismatch() {
    let mut svm = load_svm();
    let authority_a = Keypair::new();
    let authority_b = Keypair::new();
    let member = Keypair::new();
    svm.airdrop(&member.pubkey(), 10_000_000_000).unwrap();
    let org_a = init_org(&mut svm, &authority_a, "h3-4a");
    let org_b = init_org(&mut svm, &authority_b, "h3-4b");
    let membership_in_a = grant(&mut svm, &authority_a, org_a, member.pubkey(), organization::Role::Submitter);

    let (ix, _) = register_ix(org_b, member.pubkey(), membership_in_a, 4);
    let res = send(&mut svm, &member, &[ix], &[]);
    assert_error_code(&res, 6003);
}

#[test]
fn scenario_5_member_mismatch() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let member = Keypair::new();
    let impostor = Keypair::new();
    svm.airdrop(&member.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&impostor.pubkey(), 10_000_000_000).unwrap();
    let org = init_org(&mut svm, &authority, "h3-5");
    let membership = grant(&mut svm, &authority, org, member.pubkey(), organization::Role::Submitter);

    let (ix, _) = register_ix(org, impostor.pubkey(), membership, 5);
    let res = send(&mut svm, &impostor, &[ix], &[]);
    assert_error_code(&res, 6004);
}

#[test]
fn scenario_6_duplicate_cid() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let org = init_org(&mut svm, &authority, "h3-6");
    let (admin_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );

    let (ix1, _) = register_ix(org, authority.pubkey(), admin_membership, 6);
    assert_ok(&send(&mut svm, &authority, &[ix1], &[]));

    let (ix2, _) = register_ix(org, authority.pubkey(), admin_membership, 6);
    let res = send(&mut svm, &authority, &[ix2], &[]);
    assert_error_code(&res, 0);
}

#[test]
fn scenario_7_forged_membership_owner() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let org = init_org(&mut svm, &authority, "h3-7");
    let (real_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );

    let real_account = svm.get_account(&real_membership).unwrap();
    let forged_membership = Pubkey::new_unique();
    svm.set_account(
        forged_membership,
        Account {
            lamports: real_account.lamports,
            data: real_account.data.clone(),
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let (ix, _) = register_ix(org, authority.pubkey(), forged_membership, 7);
    let res = send(&mut svm, &authority, &[ix], &[]);
    assert_error_code(&res, 3007);
}

#[test]
fn scenario_8_oversized_cid_len_returns_error_not_panic() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let org = init_org(&mut svm, &authority, "h3-8");
    let (admin_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );

    let cid: [u8; 64] = {
        let mut c = [0u8; 64];
        c[0] = 0x01;
        c[1] = 0x70;
        c[2] = 0x12;
        c[3] = 0x20;
        c[4] = 8;
        c
    };
    let cid_len: u8 = 200; // > 64, would panic an unclamped `&cid[..cid_len as usize]` slice
    // PDA must match the clamped-in-program computation.
    let (film_pda, _) = {
        let digest = solana_sha256_hasher::hash(&cid[..(cid_len as usize).min(64)]);
        Pubkey::find_program_address(&[b"film", org.as_ref(), &digest.to_bytes()[..32]], &film::id())
    };
    let event_authority = Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0;
    let ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::RegisterFilm {
            cid,
            cid_len,
            byte_size: 1,
            chunking_profile: 1,
            title: "T".to_string(),
        }
        .data(),
        film::accounts::RegisterFilm {
            submitter: authority.pubkey(),
            organization: org,
            membership: admin_membership,
            film: film_pda,
            organization_program: organization::id(),
            system_program: system_program::ID,
            event_authority,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, &authority, &[ix], &[]);
    assert_error_code(&res, 6005);
}
