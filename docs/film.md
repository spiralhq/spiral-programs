# `film` program

Registers audiovisual works on-chain. Each film is a `Film` account, addressed
deterministically from its IPFS CID and the submitting organization. Depends on
`organization` (see `docs/organization.md`) for authorization: every instruction CPIs into
`organization::assert_can_submit` before touching a `Film` account.

Program ID: `6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu`.

## Accounts

### `Film`
| Field | Type | Notes |
|---|---|---|
| `organization` | `Pubkey` | Which organization submitted this film. |
| `submitter` | `Pubkey` | Signer who called `register_film`. Can manage the film's replicas and access policy while their membership stays active with role Submitter or Admin (see "Management authorization"). |
| `cid` | `[u8; 64]` | Fixed-size buffer holding the IPFS CID, zero-padded. |
| `cid_len` | `u8` | How many of `cid`'s 64 bytes are the actual CID; always `36` under the current chunking profile. |
| `byte_size` | `u64` | Size of the underlying content. |
| `chunking_profile` | `u8` | Which chunking profile produced this CID. |
| `title` | `String` (max 160 bytes) | Display title. |
| `submitted_at` | `i64` | Unix timestamp. |
| `access_policy` | `AccessPolicy` | `Public` or `OrganizationOnly`. |
| `replicas` | `[ReplicaDeclaration; 8]` | Fixed-size slots; unused ones stay at `ReplicaDeclaration::default()`. |
| `replica_count` | `u8` | How many of the 8 slots are populated. |
| `schema_version` | `u8` | Reserved for future migrations. |
| `bump` | `u8` | PDA bump. |

PDA: `["film", organization, sha256(cid[..cid_len])[..32]]` — the address is a
function of the CID itself, so submitting the same CID to the same organization
twice tries to create the same account, and the second `init` fails because the
account already exists. This is how CID de-duplication happens, with no separate
index needed.

Because Anchor evaluates `seeds` constraints in `try_accounts`, before the
handler's own `require!` checks run, the PDA-derivation code clamps the slice with
`.min(64)`: an oversized `cid_len` would otherwise panic while slicing `cid`
instead of surfacing as the handled `InvalidCidLength` error. See scenario 8 in
`tests/h3_scenarios.rs` for the regression test.

### `ReplicaDeclaration`
| Field | Type | Notes |
|---|---|---|
| `peer_id` | `[u8; 38]` | libp2p PeerID bytes of an Ed25519 identity (`00 24 08 01 12 20` + 32-byte key). |
| `peer_id_len` | `u8` | Always `38` for slots written by `declare_replica`. |
| `declared_at` | `i64` | Unix timestamp of the latest declaration, reactivation or reuse of the slot. |
| `active` | `bool` | Set to `false` by `deactivate_replica`; the slot can later be reactivated or reused. |

Implements `Default` by hand (zeroed `peer_id`, `peer_id_len = 0`, `declared_at =
0`, `active = false`) because Rust's standard library only provides a blanket
`Default` for arrays up to 32 elements, and `peer_id` is 38.

### `AccessPolicy`
Enum: `Public` or `OrganizationOnly`.

## Instructions

### `register_film(cid, cid_len, byte_size, chunking_profile, title)`
1. Validates the CID's first 4 bytes against the Spiral Chunking Profile v1
   (`0x01` CIDv1, `0x70`/`0x55` codec, `0x12` sha2-256 multihash code, `0x20`
   32-byte digest length) and that `cid_len == 36`.
2. Requires every byte of `cid[cid_len..]` to be zero (`CidPaddingNotZero`,
   6012), so one CID cannot yield the same `Film` address with different
   trailing bytes, and requires `title` to be at most 160 bytes
   (`TitleTooLong`, 6011). Both run after the CID checks, so a bad CID still
   reports 6005/6006.
3. CPIs into `organization::assert_can_submit` — this is what rejects an
   unauthorized submitter before any `Film` account is touched.
4. Writes the `Film` account fields, defaulting `access_policy` to `Public` and
   `replicas` to all-default.
5. Emits `FilmRegistered` via `emit_cpi!`.

Accounts: `submitter` (signer, payer), `organization` (read-only), `membership`,
`film` (PDA, `init`), `organization_program`, `system_program`, plus
`event_authority` and `program` from `#[event_cpi]`.

`register_film` deliberately has no `seeds` constraint on `membership`, unlike
the management instructions. The CPI into `assert_can_submit` then reports a
membership of another organization as `OrgMismatch` (6003) and one of another
member as `MemberMismatch` (6004), which H3 scenarios 4 and 5 expect. A seeds
constraint would replace both with Anchor's `ConstraintSeeds` (2006). The CPI's
`require_role` plus the owner and discriminator checks already rule out forged
accounts.

### Management accounts

`declare_replica`, `deactivate_replica` and `update_access_policy` take the same
accounts, in this order:

| Account | Constraint |
|---|---|
| `authority` | Signer. |
| `organization` | `organization::Organization`; must equal `film.organization` (`OrgMismatch`, 6003, otherwise). |
| `membership` | `organization::Membership` at PDA `["membership", organization, authority]` under the `organization` program (`seeds::program`); any other address fails with Anchor's `ConstraintSeeds` (2006). |
| `organization_program` | The `organization` program. |
| `film` | `Film`, writable. |
| `event_authority`, `program` | Added by `#[event_cpi]` for `emit_cpi!`. |

### Management authorization

Account validation runs first. `membership` must be owned by the
`organization` program (3007 otherwise) and sit at the PDA derived from
`organization` and `authority`. A membership for another organization or
another member therefore fails with `ConstraintSeeds` (2006), or with
`AccountNotInitialized` (3012) when nothing exists at the derived address. The
`has_one` on `film` rejects an `organization` other than `film.organization`
with `OrgMismatch` (6003).

Each of the three instructions then CPIs into `organization::assert_can_submit`
with `member = authority`, the same check `register_film` uses: the membership
must be active (6001) and have role Submitter or Admin (6002). The CPI also
checks organization and member (6003/6004), but the seeds constraint has already
guaranteed both, so those codes cannot come from the CPI here. On top of that,
when `authority != film.submitter` the membership role must be Admin, otherwise
the call fails with `OrgError::InsufficientRole` (6002). So a revoked or demoted
submitter loses control of the film, any Admin of the film's organization can
manage it, and nobody outside the organization can.

### `declare_replica(peer_id, peer_id_len)`
1. Authorizes the caller (see above).
2. Requires `peer_id_len == 38` and the Ed25519 PeerID prefix
   `00 24 08 01 12 20` (`InvalidPeerId` otherwise).
3. Picks a slot among the first `replica_count` slots:
   - an active slot already holding `peer_id` → `DuplicateReplica`;
   - an inactive slot holding `peer_id` → reactivated in place;
   - otherwise, while `replica_count < 8`, a new slot is appended;
   - otherwise the first inactive slot is reused, overwriting its old PeerID;
   - otherwise `ReplicaCapacityExceeded`.
4. Writes the slot with `active = true` and the current `declared_at`.
5. Emits `ReplicaDeclared`.

### `deactivate_replica(index)`
Authorizes the caller, requires `index < replica_count` (`InvalidReplicaIndex`
otherwise), sets `replicas[index].active = false` and emits
`ReplicaDeactivated`. The slot keeps its PeerID until it is reused.
Deactivating a slot that is already inactive succeeds as a no-op; the event's
`was_active` is then `false`.

### `update_access_policy(new_policy)`
Authorizes the caller, overwrites `film.access_policy` and emits
`AccessPolicyUpdated`.

## Errors (`FilmError`, codes 6005-6013)

`#[error_code(offset = 6005)]` starts the numbering at 6005, so
`InvalidCidLength` is 6005. The offset does not keep these codes apart from
`organization::OrgError`, which runs to 6010 (see below).

| Code | Variant | Meaning |
|---|---|---|
| 6005 | `InvalidCidLength` | `cid_len != 36`. |
| 6006 | `InvalidCidPrefix` | First 4 bytes don't match the Spiral Chunking Profile v1. |
| 6007 | `ReplicaCapacityExceeded` | `declare_replica` with a new PeerID while all 8 slots are active. |
| 6008 | `InvalidReplicaIndex` | `deactivate_replica` called with `index >= replica_count`. |
| 6009 | `InvalidPeerId` | `declare_replica` with a PeerID that is not a 38-byte Ed25519 libp2p identity. |
| 6010 | `DuplicateReplica` | `declare_replica` with a PeerID that already holds an active slot. |
| 6011 | `TitleTooLong` | `register_film` with a title longer than 160 bytes. |
| 6012 | `CidPaddingNotZero` | `register_film` with a non-zero byte in `cid[cid_len..]`. |
| 6013 | `ReplicaCountOverflow` | `replica_count` would overflow; unreachable while capacity is 8, kept instead of a panic. |

Custom error codes overlap across the two programs (organization uses
6000-6010, film 6005-6013): codes 6005-6010 exist in both with different
meanings, and only the failing instruction's program ID disambiguates them.
Always read the program ID before decoding a code.

New variants are only ever appended. The management instructions also return
`organization::OrgError` 6001/6002 from the `assert_can_submit` CPI, 6002 from
the Admin rule and 6003 from the `has_one` on `film`, as described under
"Management authorization". A mismatched membership surfaces as Anchor's 2006,
3007 or 3012, never as 6003/6004.

## Events

- **`FilmRegistered`** (`organization`, `submitter`, `cid`, `cid_len`, `byte_size`,
  `chunking_profile`, `submitted_at`) — emitted by `register_film`. Deliberately
  excludes `title`, `access_policy`, and the replica list: not needed by a
  low-latency "new film" listener, and any consumer that needs the full picture
  reads the `Film` account directly (or backfills via `getProgramAccounts`).
- **`ReplicaDeclared`** (`film`, `authority`, `peer_id`, `peer_id_len`, `slot`,
  `reactivated`, `replaced_peer_id`) — emitted by `declare_replica`. `reactivated` is true when an
  inactive slot holding the same PeerID was switched back on;
  `replaced_peer_id` carries the old PeerID when an inactive slot was reused for
  a different one. Because reuse overwrites the slot, these events are the only
  on-chain record of earlier PeerIDs.
- **`ReplicaDeactivated`** (`film`, `authority`, `slot`, `peer_id`, `was_active`)
  — emitted by `deactivate_replica`. `was_active` is `false` when the slot was
  already inactive, so the call changed nothing.
- **`AccessPolicyUpdated`** (`film`, `authority`, `policy`) — emitted by
  `update_access_policy`.

In the three management events, `authority` is the signer who made the change.
When it differs from `film.submitter`, an organization Admin intervened.

All events are emitted with `emit_cpi!`, so they appear as self-CPI inner
instructions rather than log lines.

## Negative-path tests (`tests/h3_scenarios.rs`)

| Scenario | Forced condition | Expected code |
|---|---|---|
| 1 | Signer has no `Membership` account at all | `3012` (Anchor native `AccountNotInitialized`; `OrgError::MembershipNotFound` = 6000 is reserved but never actually returned) |
| 2 | Membership revoked | `6001` `MembershipRevoked` |
| 3 | Role `Reader` submitting | `6002` `InsufficientRole` |
| 4 | Membership belongs to a different organization | `6003` `OrgMismatch` |
| 5 | Signer isn't `membership.member` | `6004` `MemberMismatch` |
| 6 | Duplicate CID in the same organization | `0` (`init` fails, account already exists) |
| 7 | Membership-shaped account owned by the wrong program (forged) | `3007` (Anchor native `AccountOwnedByWrongProgram`) |
| 7 (variant) | A `Film` account passed as the membership | `3007` (Anchor native `AccountOwnedByWrongProgram`) |
| 8 | `cid_len` = 200 (> 64) | `6005` `InvalidCidLength`, without panicking (regression test for the `.min(64)` clamp above) |

## Management tests

`tests/management_authz.rs` runs each management instruction against: the active
submitter (ok), the submitter after revoke (6001), the submitter demoted to
Reader (6002), another active Submitter (6002), an org Admin who is not the
submitter (ok), an Admin of another organization with its own org (6003), and
memberships at the wrong address (2006), and a membership owned by another
program (3007). It also checks the `authority` and `was_active` event fields. `tests/replicas.rs` covers PeerID
validation, duplicates, reactivation, append-before-reuse, slot reuse and
capacity.

`tests/error_docs.rs` checks that the code range stated here and in the README
matches `idl/film.json`, and that the `ReplicaCapacityExceeded` message names
`MAX_REPLICAS`.
