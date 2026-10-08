#[path = "../../../test-support/program_bytes.rs"]
mod program_bytes;

use program_bytes::program_bytes;

use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
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
    svm.set_sysvar(&Clock {
        unix_timestamp: 1_700_000_000,
        ..Default::default()
    });
    svm.add_program(organization::id(), &program_bytes("organization"))
        .unwrap();
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
fn full_membership_lifecycle() {
    let mut svm = load_svm();
    let authority = Keypair::new();
    let member = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let slug = "roles-org".to_string();
    let (org, _) =
        Pubkey::find_program_address(&[b"organization", slug.as_bytes()], &organization::id());
    let (admin_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), authority.pubkey().as_ref()],
        &organization::id(),
    );

    // initialize_organization
    let init_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::InitializeOrganization {
            slug: slug.clone(),
            name: "Roles Org".to_string(),
        }
        .data(),
        organization::accounts::InitializeOrganization {
            authority: authority.pubkey(),
            organization: org,
            membership: admin_membership,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[init_ix]);

    let acc = svm.get_account(&org).unwrap();
    let mut data: &[u8] = &acc.data;
    let org_state = organization::Organization::try_deserialize(&mut data).unwrap();
    assert_eq!(org_state.member_count, 1);
    assert_eq!(org_state.authority, authority.pubkey());

    // grant_membership (Reader)
    let (member_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), member.pubkey().as_ref()],
        &organization::id(),
    );
    let grant_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::GrantMembership {
            role: organization::Role::Reader,
        }
        .data(),
        organization::accounts::GrantMembership {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member: member.pubkey(),
            membership: member_membership,
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
    send(&mut svm, &authority, &[grant_ix]);

    let acc = svm.get_account(&org).unwrap();
    let mut data: &[u8] = &acc.data;
    let org_state = organization::Organization::try_deserialize(&mut data).unwrap();
    assert_eq!(org_state.member_count, 2);

    // update_role: Reader -> Submitter
    let update_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::UpdateRole {
            new_role: organization::Role::Submitter,
        }
        .data(),
        organization::accounts::UpdateRole {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member: member.pubkey(),
            membership: member_membership,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[update_ix]);

    let acc = svm.get_account(&member_membership).unwrap();
    let mut data: &[u8] = &acc.data;
    let state = organization::Membership::try_deserialize(&mut data).unwrap();
    assert_eq!(state.role, organization::Role::Submitter);
    assert_eq!(state.status, organization::MembershipStatus::Active);

    // revoke_membership
    let revoke_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::RevokeMembership {}.data(),
        organization::accounts::RevokeMembership {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member: member.pubkey(),
            membership: member_membership,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[revoke_ix]);

    let acc = svm.get_account(&member_membership).unwrap();
    let mut data: &[u8] = &acc.data;
    let state = organization::Membership::try_deserialize(&mut data).unwrap();
    assert_eq!(state.status, organization::MembershipStatus::Revoked);
    assert!(state.revoked_at > 0);

    // transfer_authority: authority -> another active Admin
    let successor = Keypair::new();
    let (successor_membership, _) = Pubkey::find_program_address(
        &[b"membership", org.as_ref(), successor.pubkey().as_ref()],
        &organization::id(),
    );
    let grant_admin_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::GrantMembership {
            role: organization::Role::Admin,
        }
        .data(),
        organization::accounts::GrantMembership {
            authority: authority.pubkey(),
            organization: org,
            admin_membership,
            member: successor.pubkey(),
            membership: successor_membership,
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
    send(&mut svm, &authority, &[grant_admin_ix]);

    let transfer_ix = Instruction::new_with_bytes(
        organization::id(),
        &organization::instruction::TransferAuthority {}.data(),
        organization::accounts::TransferAuthority {
            authority: authority.pubkey(),
            organization: org,
            new_authority: successor.pubkey(),
            new_authority_membership: successor_membership,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &organization::id(),
            )
            .0,
            program: organization::id(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, &authority, &[transfer_ix]);

    let acc = svm.get_account(&org).unwrap();
    let mut data: &[u8] = &acc.data;
    let org_state = organization::Organization::try_deserialize(&mut data).unwrap();
    assert_eq!(org_state.authority, successor.pubkey());
}
