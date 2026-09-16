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
    let org_bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/organization.so"));
    let film_bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/film.so"));
    svm.add_program(organization::id(), org_bytes).unwrap();
    svm.add_program(film::id(), film_bytes).unwrap();
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
        &[b"membership", organization_pda.as_ref(), authority.pubkey().as_ref()],
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
        for i in 4..36 {
            c[i] = i as u8;
        }
        c
    };
    let (film_pda, _) = {
        let digest = solana_sha256_hasher::hash(&cid[..36]);
        Pubkey::find_program_address(&[b"film", organization_pda.as_ref(), &digest.to_bytes()[..32]], &film::id())
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
