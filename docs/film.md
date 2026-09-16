# `film` program

Registers audiovisual works on-chain. Each film is a `Film` account, addressed
deterministically from its IPFS CID and the submitting organization. Depends on
`organization` (see `docs/organization.md`) for authorization: `register_film` CPIs
into `organization::assert_can_submit` before creating a `Film` account.

Program ID: `6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu`.

## Accounts

### `Film`
| Field | Type | Notes |
|---|---|---|
| `organization` | `Pubkey` | Which organization submitted this film. |
| `submitter` | `Pubkey` | Signer who called `register_film`; the only signer allowed on `declare_replica`, `deactivate_replica`, `update_access_policy` (`has_one = submitter`). |
| `cid` | `[u8; 64]` | Fixed-size buffer holding the IPFS CID, zero-padded. |
| `cid_len` | `u8` | How many of `cid`'s 64 bytes are the actual CID; always `36` under the current chunking profile. |
| `byte_size` | `u64` | Size of the underlying content. |
| `chunking_profile` | `u8` | Which chunking profile produced this CID. |
| `title` | `String` (max 160) | Display title. |
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
| `peer_id` | `[u8; 38]` | Fixed-size buffer for an IPFS peer ID. |
| `peer_id_len` | `u8` | How many of the 38 bytes are meaningful. |
| `declared_at` | `i64` | Unix timestamp. |
| `active` | `bool` | Set to `false` by `deactivate_replica`, never removed from the array. |

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
2. CPIs into `organization::assert_can_submit` — this is what rejects an
   unauthorized submitter before any `Film` account is touched.
3. Writes the `Film` account fields, defaulting `access_policy` to `Public` and
   `replicas` to all-default.
4. Emits `FilmRegistered` via `emit_cpi!`.

### `declare_replica(peer_id, peer_id_len)`
Submitter-only. Requires `replica_count < 8` (`ReplicaCapacityExceeded` otherwise),
writes a new `ReplicaDeclaration` into the next free slot, increments
`replica_count`.

### `deactivate_replica(index)`
Submitter-only. Requires `index < replica_count` (`InvalidReplicaIndex`
otherwise), sets `replicas[index].active = false`. The slot itself is never
cleared or reused.

### `update_access_policy(new_policy)`
Submitter-only. Overwrites `film.access_policy`.

## Errors (`FilmError`, codes 6005-6008)

Explicit discriminants continue the numbering from `organization::OrgError`
(6000-6004): `InvalidCidLength = 5` becomes error code 6005.

| Code | Variant | Meaning |
|---|---|---|
| 6005 | `InvalidCidLength` | `cid_len != 36`. |
| 6006 | `InvalidCidPrefix` | First 4 bytes don't match the Spiral Chunking Profile v1. |
| 6007 | `ReplicaCapacityExceeded` | `declare_replica` called with all 8 slots full. |
| 6008 | `InvalidReplicaIndex` | `deactivate_replica` called with `index >= replica_count`. |

## Events

- **`FilmRegistered`** (`organization`, `submitter`, `cid`, `cid_len`, `byte_size`,
  `chunking_profile`, `submitted_at`) — emitted by `register_film`. Deliberately
  excludes `title`, `access_policy`, and the replica list: not needed by a
  low-latency "new film" listener, and any consumer that needs the full picture
  reads the `Film` account directly (or backfills via `getProgramAccounts`).

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
| 8 | `cid_len` = 200 (> 64) | `6005` `InvalidCidLength`, without panicking (regression test for the `.min(64)` clamp above) |
