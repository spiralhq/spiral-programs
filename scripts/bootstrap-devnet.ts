import * as path from "node:path";
import {
  createClient,
  createKeyPairFromPrivateKeyBytes,
  fetchEncodedAccount,
  getAddressFromPublicKey,
  getProgramDerivedAddress,
  type Address,
} from "@solana/kit";
import { solanaRpc } from "@solana/kit-plugin-rpc";
import { signerFromFile } from "@solana/kit-plugin-signer";
import { fetchMaybeMembership } from "./generated/organization/accounts/membership.ts";
import { getGrantMembershipInstructionAsync } from "./generated/organization/instructions/grantMembership.ts";
import { getInitializeOrganizationInstructionAsync } from "./generated/organization/instructions/initializeOrganization.ts";
import { getRevokeMembershipInstructionAsync } from "./generated/organization/instructions/revokeMembership.ts";
import { findMembershipPda } from "./generated/organization/pdas/membership.ts";
import { findOrganizationPda } from "./generated/organization/pdas/organization.ts";
import { ORGANIZATION_PROGRAM_ADDRESS } from "./generated/organization/programs/organization.ts";
import { MembershipStatus } from "./generated/organization/types/membershipStatus.ts";
import { Role } from "./generated/organization/types/role.ts";

const RPC_URL = process.env.SPIRAL_DEVNET_RPC ?? "https://api.devnet.solana.com";
// See docs/bootstrap-script-notes.md#ws-url-override.
const WS_URL = process.env.SPIRAL_DEVNET_WS;
const KEYS_DIR = path.resolve(import.meta.dirname, "..", "keys");

/** Same seed always produces the same keypair, so re-running this script is idempotent. */
function deterministicSeed(seed: string): Uint8Array {
  const hash = new Uint8Array(32);
  const bytes = Buffer.from(seed, "utf8");
  for (let i = 0; i < 32; i++) hash[i] = bytes[i % bytes.length]! ^ i;
  return hash;
}

/** See docs/bootstrap-script-notes.md#deterministic-h3-fixture-addresses. */
async function deterministicAddress(seed: string): Promise<Address> {
  const { publicKey } = await createKeyPairFromPrivateKeyBytes(deterministicSeed(seed));
  return getAddressFromPublicKey(publicKey);
}

async function organizationEventAccounts() {
  const [eventAuthority] = await getProgramDerivedAddress({
    programAddress: ORGANIZATION_PROGRAM_ADDRESS,
    seeds: [new TextEncoder().encode("__event_authority")],
  });
  return { eventAuthority, program: ORGANIZATION_PROGRAM_ADDRESS };
}

async function revokeH3Membership(
  client: SpiralBootstrapClient,
  accounts: {
    organization: Address;
    member: Address;
    membership: Address;
  },
): Promise<void> {
  const instruction = await getRevokeMembershipInstructionAsync({
    authority: client.payer,
    organization: accounts.organization,
    member: accounts.member,
    membership: accounts.membership,
    ...(await organizationEventAccounts()),
  });
  await client.sendTransaction([instruction]);
}

type SpiralBootstrapClient = Awaited<ReturnType<typeof createBootstrapClient>>;

function createBootstrapClient(keypairPath: string) {
  return createClient()
    .use(signerFromFile(keypairPath))
    .use(solanaRpc(WS_URL ? { rpcUrl: RPC_URL, rpcSubscriptionsUrl: WS_URL } : { rpcUrl: RPC_URL }));
}

async function main() {
  const client = await createBootstrapClient(path.join(KEYS_DIR, "bootstrap-payer-keypair.json"));

  const slug = "spiralhq-devnet";
  const [orgPda] = await findOrganizationPda({ slug });

  const existing = await fetchEncodedAccount(client.rpc, orgPda);
  if (existing.exists) {
    console.log(`Organization "${slug}" already exists at ${orgPda} — skipping init.`);
  } else {
    const instruction = await getInitializeOrganizationInstructionAsync({
      authority: client.payer,
      slug,
      name: "Spiral HQ (Devnet)",
    });
    await client.sendTransaction([instruction]);
    console.log(`Created organization "${slug}" at ${orgPda}.`);
  }

  // H3 negative-scenario fixtures for the evaluation harness.
  const scenarios: Array<{ name: string; role: Role | null }> = [
    { name: "h3-no-membership", role: null },
    { name: "h3-revoked", role: Role.Submitter },
    { name: "h3-reader-role", role: Role.Reader },
  ];

  for (const scenario of scenarios) {
    const memberAddress = await deterministicAddress(scenario.name);
    if (scenario.role === null) continue; // intentionally never granted

    const [membershipPda] = await findMembershipPda({
      organization: orgPda,
      member: memberAddress,
    });
    const already = await fetchEncodedAccount(client.rpc, membershipPda);
    if (already.exists) {
      // See docs/bootstrap-script-notes.md#the-h3-revoked-repair-path.
      if (scenario.name === "h3-revoked") {
        const membership = await fetchMaybeMembership(client.rpc, membershipPda);
        if (!membership.exists) {
          throw new Error(`Membership account ${membershipPda} exists on-chain but failed to decode.`);
        }
        if (membership.data.status === MembershipStatus.Active) {
          console.log(
            `Membership for ${scenario.name} exists but is still Active — finishing revoke.`,
          );
          await revokeH3Membership(client, {
            organization: orgPda,
            member: memberAddress,
            membership: membershipPda,
          });
          console.log(`Revoked membership for ${scenario.name}.`);
        } else {
          console.log(`Membership for ${scenario.name} already exists and is Revoked — skipping.`);
        }
      } else {
        console.log(`Membership for ${scenario.name} already exists — skipping grant.`);
      }
      continue;
    }

    const grantInstruction = await getGrantMembershipInstructionAsync({
      authority: client.payer,
      organization: orgPda,
      member: memberAddress,
      role: scenario.role,
      ...(await organizationEventAccounts()),
    });
    await client.sendTransaction([grantInstruction]);
    console.log(`Granted ${Role[scenario.role]} to ${scenario.name} (${memberAddress}).`);

    if (scenario.name === "h3-revoked") {
      await revokeH3Membership(client, {
        organization: orgPda,
        member: memberAddress,
        membership: membershipPda,
      });
      console.log(`Revoked membership for ${scenario.name}.`);
    }
  }

  console.log("Bootstrap complete.");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
