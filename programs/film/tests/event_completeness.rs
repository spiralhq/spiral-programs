use film::{AccessPolicy, Film, FilmRegistered, ReplicaDeclaration};

/// `FilmRegistered` must be a pure field-for-field subset of `Film`.
#[test]
fn film_registered_event_is_a_subset_of_film_account() {
    let film = Film {
        organization: anchor_lang::prelude::Pubkey::new_unique(),
        submitter: anchor_lang::prelude::Pubkey::new_unique(),
        cid: [7u8; 64],
        cid_len: 36,
        byte_size: 999_999,
        chunking_profile: 1,
        title: "guardian test".to_string(),
        submitted_at: 1_700_000_000,
        access_policy: AccessPolicy::Public,
        replicas: [ReplicaDeclaration::default(); 8],
        replica_count: 0,
        schema_version: 1,
        bump: 255,
    };

    let event = FilmRegistered {
        organization: film.organization,
        submitter: film.submitter,
        cid: film.cid,
        cid_len: film.cid_len,
        byte_size: film.byte_size,
        chunking_profile: film.chunking_profile,
        submitted_at: film.submitted_at,
    };

    assert_eq!(event.organization, film.organization);
    assert_eq!(event.submitter, film.submitter);
    assert_eq!(event.cid, film.cid);
    assert_eq!(event.cid_len, film.cid_len);
    assert_eq!(event.byte_size, film.byte_size);
    assert_eq!(event.chunking_profile, film.chunking_profile);
    assert_eq!(event.submitted_at, film.submitted_at);
}
