#[path = "../../../test-support/program_bytes.rs"]
mod program_bytes;

use program_bytes::program_bytes;

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn load_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let org_bytes = program_bytes("organization");
    let film_bytes = program_bytes("film");
    svm.add_program(organization::id(), &org_bytes).unwrap();
    svm.add_program(film::id(), &film_bytes).unwrap();
    svm
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ixs: &[Instruction]) {
    svm.expire_blockhash();
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &bh);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "transaction failed: {:?}", res.err());
}

#[test]
fn register_film_succeeds_via_cpi_to_organization() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let slug = "spiralhq".to_string();
    let (organization_pda, _) =
        Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let (membership_pda, _) = Pubkey::find_program_address(
        &[
            b"membership",
            organization_pda.as_ref(),
            authority.pubkey().as_ref(),
        ],
        &organization::id(),
    );

    let init_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.clone(),
            name: "Spiral HQ".to_string(),
        }
        .data(),
        organization::accounts::InitializeOrganization {
            authority: authority.pubkey(),
            organization: organization_pda,
            membership: membership_pda,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[init_ix]);

    let cid: [u8; 64] = {
        let mut c = [0u8; 64];
        c[0] = 0x01;
        c[1] = 0x70;
        c[2] = 0x12;
        c[3] = 0x20;
        for (i, b) in c.iter_mut().enumerate().take(36).skip(4) {
            *b = i as u8;
        }
        c
    };
    let (film_pda, _) = {
        let digest = solana_sha256_hasher::hash(&cid[..36]);
        Pubkey::find_program_address(
            &[b"film", organization_pda.as_ref(), &digest.to_bytes()[..32]],
            &film::id(),
        )
    };

    let register_ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::RegisterFilm {
            cid,
            cid_len: 36,
            byte_size: 123_456,
            chunking_profile: 1,
            title: "A Trip to the Moon".to_string(),
        }
        .data(),
        film::accounts::RegisterFilm {
            submitter: authority.pubkey(),
            organization: organization_pda,
            membership: membership_pda,
            film: film_pda,
            organization_program: organization::id(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &film::id()).0,
            program: film::id(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[register_ix]);

    let film_account = svm.get_account(&film_pda).unwrap();
    let mut data: &[u8] = &film_account.data;
    let film_state = film::Film::try_deserialize(&mut data).unwrap();
    assert_eq!(film_state.organization, organization_pda);
    assert_eq!(film_state.submitter, authority.pubkey());
    assert_eq!(film_state.cid, cid);
    assert_eq!(film_state.cid_len, 36);
}

#[allow(clippy::result_large_err)]
fn try_register(
    svm: &mut LiteSVM,
    authority: &Keypair,
    org: Pubkey,
    membership: Pubkey,
    cid: [u8; 64],
    cid_len: u8,
    title: String,
) -> litesvm::types::TransactionResult {
    let digest = solana_sha256_hasher::hash(&cid[..(cid_len as usize).min(64)]);
    let (film_pda, _) = Pubkey::find_program_address(
        &[b"film", org.as_ref(), &digest.to_bytes()[..32]],
        &film::id(),
    );
    let ix = Instruction::new_with_bytes(
        film::id(),
        &film::instruction::RegisterFilm {
            cid,
            cid_len,
            byte_size: 1,
            chunking_profile: 1,
            title,
        }
        .data(),
        film::accounts::RegisterFilm {
            submitter: authority.pubkey(),
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
    svm.expire_blockhash();
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&authority.pubkey()), &bh);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[authority]).unwrap();
    svm.send_transaction(tx)
}

fn custom_code(res: &litesvm::types::TransactionResult) -> Option<u32> {
    match &res.as_ref().err()?.err {
        solana_transaction_error::TransactionError::InstructionError(
            _,
            solana_instruction::error::InstructionError::Custom(code),
        ) => Some(*code),
        _ => None,
    }
}

fn setup_org(svm: &mut LiteSVM, authority: &Keypair) -> (Pubkey, Pubkey) {
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    let slug = "titles";
    let (org, _) =
        Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let (membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );
    let ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.to_string(),
            name: "Titles".to_string(),
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
    send(svm, authority, &[ix]);
    (org, membership)
}

fn valid_cid(seed: u8) -> [u8; 64] {
    let mut c = [0u8; 64];
    c[..4].copy_from_slice(&[0x01, 0x70, 0x12, 0x20]);
    c[4] = seed;
    c
}

#[test]
fn title_longer_than_160_bytes_is_refused() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let (org, membership) = setup_org(&mut svm, &authority);
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        valid_cid(1),
        36,
        "t".repeat(161),
    );
    assert_eq!(custom_code(&res), Some(6011), "got {:?}", res.err());
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        valid_cid(2),
        36,
        "é".repeat(81),
    );
    assert_eq!(custom_code(&res), Some(6011), "got {:?}", res.err());
}

#[test]
fn title_of_160_bytes_is_accepted() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let (org, membership) = setup_org(&mut svm, &authority);
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        valid_cid(1),
        36,
        "t".repeat(160),
    );
    assert!(res.is_ok(), "got {:?}", res.err());
}

#[test]
fn cid_errors_take_precedence_over_title_length() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let (org, membership) = setup_org(&mut svm, &authority);
    let long = "t".repeat(161);
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        valid_cid(1),
        35,
        long.clone(),
    );
    assert_eq!(custom_code(&res), Some(6005));
    let mut bad_prefix = valid_cid(1);
    bad_prefix[0] = 0x00;
    let res = try_register(&mut svm, &authority, org, membership, bad_prefix, 36, long);
    assert_eq!(custom_code(&res), Some(6006));
}

#[test]
fn nonzero_bytes_after_cid_len_are_refused() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let (org, membership) = setup_org(&mut svm, &authority);
    let mut padded = valid_cid(1);
    padded[63] = 0xff;
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        padded,
        36,
        "t".into(),
    );
    assert_eq!(custom_code(&res), Some(6012), "got {:?}", res.err());
    padded[63] = 0;
    padded[36] = 1;
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        padded,
        36,
        "t".into(),
    );
    assert_eq!(custom_code(&res), Some(6012), "got {:?}", res.err());
    let res = try_register(
        &mut svm,
        &authority,
        org,
        membership,
        valid_cid(1),
        36,
        "t".into(),
    );
    assert!(res.is_ok(), "got {:?}", res.err());
}
