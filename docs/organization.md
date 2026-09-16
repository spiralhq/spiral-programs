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
| `slug` | `String` (max 32) | Human-readable identifier; also the PDA seed. |
| `name` | `String` (max 96) | Display name. |
| `created_at` | `i64` | Unix timestamp, set once at `initialize_organization`. |
| `member_count` | `u32` | Incremented on every `grant_membership`. Not decremented on revoke. |
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

### `grant_membership(role)`
Caller must hold an Active Admin `Membership` (checked via `require_role`, see
below). Creates a new `Membership` for `member` with the given `role`, and
increments `organization.member_count`.

### `update_role(new_role)`
Admin-only. Overwrites `role` on an existing `Membership`.

### `revoke_membership()`
Admin-only. Sets `status = Revoked` and records `revoked_at`. Emits
`MembershipRevoked`. Membership accounts are never closed/deleted — a revoked
membership still exists on-chain, just inactive.

### `transfer_authority()`
Changes `organization.authority`. Restricted to the current authority via the
`has_one = authority` account constraint (Anchor rejects the transaction before the
handler runs if the signer doesn't match).

### `assert_can_submit()`
No state mutation — a pure authorization check. Meant to be invoked via CPI from
`film::register_film` before it creates a `Film` account. Requires an Active
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

1. `membership.status == Active` → else `MembershipRevoked`
2. `membership.role` is in the instruction's allowed set → else `InsufficientRole`
3. `membership.organization` matches the expected organization → else `OrgMismatch`
4. `membership.member` matches the transaction signer → else `MemberMismatch`

A missing `Membership` account entirely (no account ever created) is not one of
these four cases — Anchor's own `AccountNotInitialized` check rejects the
transaction before `require_role` is ever called, when it tries to deserialize an
account that doesn't exist.

## Errors (`OrgError`, codes 6000-6004)

| Code | Variant | Meaning |
|---|---|---|
| 6000 | `MembershipNotFound` | Reserved, never actually returned — kept only so the rest of the enum keeps its numbering. The real failure for "no membership account" is Anchor's native `AccountNotInitialized` (3012). |
| 6001 | `MembershipRevoked` | Membership exists but `status == Revoked`. |
| 6002 | `InsufficientRole` | Role not in the instruction's allowed set. |
| 6003 | `OrgMismatch` | Membership belongs to a different organization. |
| 6004 | `MemberMismatch` | Signer isn't `membership.member`. |

Removing or reordering a variant renumbers everything after it — anything hardcoding
these codes (tests, clients) breaks silently if that happens without updating them
too.

## Events

- **`MembershipRevoked`** (`organization`, `member`, `revoked_at`) — emitted by
  `revoke_membership` via `emit_cpi!`. Every field is already present on
  `Membership` itself; the event exists so a client can react without re-fetching
  the account, not to carry new data.

## Cross-program surface used by `film`

`film::register_film` CPIs into `assert_can_submit` before creating a `Film`
account. See `programs/film/src/instructions/register_film.rs` for the caller side.
