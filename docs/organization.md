# `organization` program

Owns organizations, memberships, and role-based access control. Other programs
(currently `film`) authorize actions against an organization by CPI-calling
`assert_can_submit`, rather than reading `Membership` state directly.

Program ID: `2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw`.

## Accounts

### `Organization`
| Field | Type | Notes |
|---|---|---|
| `authority` | `Pubkey` | Current owner; the only signer allowed to call `transfer_authority`. |
| `slug` | `String` (1-32 bytes, `[a-z0-9-]`) | Human-readable identifier; also the PDA seed. |
| `name` | `String` (1-96 bytes) | Display name. |
| `created_at` | `i64` | Unix timestamp, set once at `initialize_organization`. |
| `member_count` | `u32` | Total memberships ever granted, including the authority's own. Incremented on every `grant_membership`, never decremented: revoked memberships stay counted, and re-granting is impossible, so it is not the number of active members. Overflow fails with `MemberCountOverflow` (6009). |
| `schema_version` | `u8` | Reserved for future migrations. |
| `bump` | `u8` | PDA bump, cached to avoid recomputing it in later instructions. |

PDA: `["organization", slug]`.

### `Membership`
| Field | Type | Notes |
|---|---|---|
| `organization` | `Pubkey` | Which `Organization` this membership belongs to. |
| `member` | `Pubkey` | The account holder. |
| `role` | `Role` | `Reader`, `Submitter`, or `Admin`. |
| `status` | `MembershipStatus` | `Active` or `Revoked`. |
| `granted_by` | `Pubkey` | Admin who granted this membership. |
| `granted_at` / `revoked_at` | `i64` | Unix timestamps; `revoked_at` is `0` until revoked. |
| `bump` | `u8` | PDA bump. |

PDA: `["membership", organization, member]` — deriving the address only from
`(organization, member)` means each member can hold exactly one `Membership` per
organization, and any client can compute the address without an on-chain lookup.

## Instructions

### `initialize_organization(slug, name)`
Creates the `Organization` account and, in the same instruction, an Active Admin
`Membership` for the caller (`authority`). This is deliberate bootstrapping: every
other membership-mutating instruction requires an existing Admin `Membership` to
authorize the call, so without this the very first membership on a fresh
organization could never be created.

Argument checks:
- `slug` must be 1-32 bytes, all in `[a-z0-9-]`, otherwise `SlugInvalid` (6005).
  The check runs inside the PDA seed expression, because Anchor derives the
  `organization` address before the handler runs and the derivation itself
  panics on a seed longer than 32 bytes. The IDL still records the seed as the
  `slug` argument.
- `name` must be 1-96 bytes: empty fails with `NameEmpty` (6010), longer than 96
  with `NameTooLong` (6006). This check runs at
  the start of the handler; the failed transaction rolls back both `init`s, so
  no account is left behind.

### `grant_membership(role)`
Caller must hold an Active Admin `Membership` (checked via `require_role`, see
below). Creates a new `Membership` for `member` with the given `role`, and
increments `organization.member_count`.

### `update_role(new_role)`
Admin-only. Overwrites `role` on an existing `Membership`. The membership of
`organization.authority` cannot be changed (`AuthorityProtected`, 6008), even by
the authority itself.

### `revoke_membership()`
Admin-only. Sets `status = Revoked` and records `revoked_at`. Emits
`MembershipRevoked`. Membership accounts are never closed/deleted — a revoked
membership still exists on-chain, just inactive.

The target must be Active (`MembershipNotActive`, 6007), so a second revoke
cannot overwrite the original `revoked_at` or emit a second event. The
membership of `organization.authority` cannot be revoked (`AuthorityProtected`,
6008).

Together these rules keep the organization from losing its last Admin: the
authority's membership starts as an Active Admin, cannot be demoted or revoked,
and authority can only move to another Active Admin. So whoever holds
`organization.authority` is always an Active Admin.

Re-granting a revoked member is not supported: `grant_membership` uses `init`
on the same PDA, which already exists, so it fails. A revoked member stays
revoked.

### `transfer_authority()`
Changes `organization.authority`. Restricted to the current authority via the
`has_one = authority` account constraint (Anchor rejects the transaction before the
handler runs if the signer doesn't match).

Takes `new_authority_membership` at PDA `["membership", organization,
new_authority]`, and the new authority must hold an Active Admin membership
there (checked with `require_role`). A key with no membership fails with
`AccountNotInitialized` (3012), a revoked member with `MembershipRevoked` (6001)
and a Reader or Submitter with `InsufficientRole` (6002). After the transfer the
protection below moves to the new authority, and the old authority becomes an
ordinary Admin that can be demoted or revoked.

### `assert_can_submit()`
No state mutation — a pure authorization check. Meant to be invoked via CPI from
`film::register_film` before it creates a `Film` account, and from `film`'s
replica and access-policy instructions before they change one. Requires an Active
`Membership` with role `Submitter` or `Admin`.

Requiring `member` to be a real `Signer` (not just a pubkey) is what makes this
CPI-safe: a forged `Membership` account still can't be used to submit on someone
else's behalf, because the transaction itself has to be signed by the claimed
member.

The account struct deliberately has **no** `seeds`/`bump` constraint on
`membership`. Adding one would make Anchor reject an org- or member-mismatched
account before `require_role` ever runs, collapsing those two failure modes into a
single generic Anchor error instead of the distinct `OrgMismatch` / `MemberMismatch`
codes below. `require_role` alone is sufficient authorization: the owner and
discriminator checks that `Account<'info, Membership>` already performs rule out
forged accounts on their own.

## Authorization: `require_role`

Every role-gated instruction funnels through one function, `authz::require_role`,
which checks, in order:

1. `membership.status` is matched exhaustively; `Active` passes, `Revoked` → `MembershipRevoked`
2. `membership.role` is in the instruction's allowed set → else `InsufficientRole`
3. `membership.organization` matches the expected organization → else `OrgMismatch`
4. `membership.member` matches the transaction signer → else `MemberMismatch`

A missing `Membership` account entirely (no account ever created) is not one of
these four cases — Anchor's own `AccountNotInitialized` check rejects the
transaction before `require_role` is ever called, when it tries to deserialize an
account that doesn't exist.

## Errors (`OrgError`, codes 6000-6010)

| Code | Variant | Meaning |
|---|---|---|
| 6000 | `MembershipNotFound` | Reserved, never actually returned — kept only so the rest of the enum keeps its numbering. The real failure for "no membership account" is Anchor's native `AccountNotInitialized` (3012). |
| 6001 | `MembershipRevoked` | The membership checked by `require_role` (the signer's own) has `status == Revoked`. |
| 6002 | `InsufficientRole` | Role not in the instruction's allowed set. |
| 6003 | `OrgMismatch` | Membership belongs to a different organization. |
| 6004 | `MemberMismatch` | Signer isn't `membership.member`. |
| 6005 | `SlugInvalid` | `initialize_organization` slug is empty, longer than 32 bytes, or has a byte outside `[a-z0-9-]`. |
| 6006 | `NameTooLong` | `initialize_organization` name is longer than 96 bytes. |
| 6007 | `MembershipNotActive` | `revoke_membership` on a target membership that is already revoked. |
| 6008 | `AuthorityProtected` | `revoke_membership` or `update_role` targeting the membership of `organization.authority`. |
| 6009 | `MemberCountOverflow` | `grant_membership` when `member_count` is already `u32::MAX`. |
| 6010 | `NameEmpty` | `initialize_organization` with an empty name. |

`transfer_authority` also returns 6001/6002 (and 6003/6004, which the seeds
constraint makes unreachable) from `require_role` on the new authority's
membership.

`tests/error_docs.rs` checks that the code range stated here and in the README matches `idl/organization.json`.

Custom error codes overlap across the two programs (organization uses
6000-6010, film 6005-6013): codes 6005-6010 exist in both with different
meanings, and only the failing instruction's program ID disambiguates them.
Always read the program ID before decoding a code.

New variants are only ever appended. Removing or reordering a variant renumbers everything after it — anything hardcoding
these codes (tests, clients) breaks silently if that happens without updating them
too.

## Events

- **`MembershipRevoked`** (`organization`, `member`, `revoked_at`) — emitted by
  `revoke_membership` via `emit_cpi!`. Every field is already present on
  `Membership` itself; the event exists so a client can react without re-fetching
  the account, not to carry new data.

- **`MembershipGranted`** (`organization`, `member`, `role`, `granted_by`,
  `granted_at`) — emitted by `grant_membership`.
- **`MembershipRoleUpdated`** (`organization`, `member`, `authority`, `old_role`,
  `new_role`) — emitted by `update_role`. `authority` is the signing Admin, as in
  `film`'s management events. `old_role` is the only on-chain record of the
  previous role.
- **`AuthorityTransferred`** (`organization`, `old_authority`, `new_authority`) —
  emitted by `transfer_authority`.

All four events use `emit_cpi!`, so `grant_membership`, `update_role`,
`revoke_membership` and `transfer_authority` take the extra `event_authority`
and `program` accounts that `#[event_cpi]` adds.

## Cross-program surface used by `film`

`film::register_film` takes the `Organization` account read-only and CPIs into `assert_can_submit` before creating a `Film`
account, and `declare_replica`, `deactivate_replica` and `update_access_policy`
do the same before changing one (with `member` = the signing authority). See
`programs/film/src/instructions/register_film.rs` and
`programs/film/src/instructions/manage.rs` for the caller side.
