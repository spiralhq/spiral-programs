import * as fs from "node:fs";
import * as path from "node:path";
import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";

const RPC_URL = process.env.SPIRAL_DEVNET_RPC ?? "https://api.devnet.solana.com";
const KEYS_DIR = path.resolve(import.meta.dirname, "..", "keys");
const IDL_DIR = path.resolve(import.meta.dirname, "..", "target", "idl");

function loadKeypair(name: string): Keypair {
  const raw = JSON.parse(fs.readFileSync(path.join(KEYS_DIR, `${name}-keypair.json`), "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

function loadIdl(name: string): anchor.Idl {
  return JSON.parse(fs.readFileSync(path.join(IDL_DIR, `${name}.json`), "utf8"));
}

/** Same seed always produces the same keypair, so re-running this script is idempotent. */
function deterministicKeypair(seed: string): Keypair {
  const hash = new Uint8Array(32);
  const bytes = Buffer.from(seed, "utf8");
  for (let i = 0; i < 32; i++) hash[i] = bytes[i % bytes.length] ^ i;
  return Keypair.fromSeed(hash);
}

/** Shared by the normal grant-then-revoke path and the repair path below, which
 * finishes a revoke left incomplete by a prior interrupted run. */
async function revokeH3Membership(
  orgProgram: anchor.Program,
  accounts: {
    authority: PublicKey;
    organization: PublicKey;
    adminMembership: PublicKey;
    member: PublicKey;
    membership: PublicKey;
  },
): Promise<void> {
  const [eventAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    orgProgram.programId,
  );
  await orgProgram.methods
    .revokeMembership()
    .accounts({ ...accounts, eventAuthority, program: orgProgram.programId })
    .rpc();
}

async function main() {
  const connection = new anchor.web3.Connection(RPC_URL, "confirmed");
  const payer = loadKeypair("bootstrap-payer");
  const wallet = new anchor.Wallet(payer);
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  anchor.setProvider(provider);

  const orgProgram = new anchor.Program(loadIdl("organization"), provider);
  const filmProgram = new anchor.Program(loadIdl("film"), provider);
  void filmProgram; // present for parity with future scripts that submit films here too

  const slug = "spiralhq-devnet";
  const [orgPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("organization"), Buffer.from(slug)],
    orgProgram.programId,
  );
  const [adminMembershipPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("membership"), orgPda.toBuffer(), payer.publicKey.toBuffer()],
    orgProgram.programId,
  );

  const existing = await connection.getAccountInfo(orgPda);
  if (existing) {
    console.log(`Organization "${slug}" already exists at ${orgPda.toBase58()} — skipping init.`);
  } else {
    await orgProgram.methods
      .initializeOrganization(slug, "Spiral HQ (Devnet)")
      .accounts({
        authority: payer.publicKey,
        organization: orgPda,
        membership: adminMembershipPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log(`Created organization "${slug}" at ${orgPda.toBase58()}.`);
  }

  // H3 negative-scenario fixtures: deterministic keypairs, granted (or deliberately
  // NOT granted) membership so an evaluation harness always finds the same accounts
  // in the same state without depending on manually-accumulated history.
  const scenarios: Array<{ name: string; role: "Reader" | "Submitter" | "Admin" | null }> = [
    { name: "h3-no-membership", role: null },
    { name: "h3-revoked", role: "Submitter" },
    { name: "h3-reader-role", role: "Reader" },
  ];

  for (const scenario of scenarios) {
    const kp = deterministicKeypair(scenario.name);
    if (scenario.role === null) continue; // intentionally never granted

    const [membershipPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("membership"), orgPda.toBuffer(), kp.publicKey.toBuffer()],
      orgProgram.programId,
    );
    const already = await connection.getAccountInfo(membershipPda);
    if (already) {
      // Existence alone doesn't mean h3-revoked is fully provisioned: a prior run
      // may have granted membership and died before the revoke landed. Decode the
      // actual status and finish the revoke if needed.
      if (scenario.name === "h3-revoked") {
        // `anchor.Idl`'s `accounts` field is optional, so TS can't infer account
        // names here; the cast just satisfies the type checker.
        const membershipAccount = orgProgram.account as unknown as {
          membership: { fetch(address: PublicKey): Promise<{ status: Record<string, unknown> }> };
        };
        const membership = await membershipAccount.membership.fetch(membershipPda);
        const status = membership.status;
        if ("active" in status) {
          console.log(`Membership for ${scenario.name} exists but is still Active — finishing revoke.`);
          await revokeH3Membership(orgProgram, {
            authority: payer.publicKey,
            organization: orgPda,
            adminMembership: adminMembershipPda,
            member: kp.publicKey,
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
    await orgProgram.methods
      .grantMembership({ [scenario.role.toLowerCase()]: {} })
      .accounts({
        authority: payer.publicKey,
        organization: orgPda,
        adminMembership: adminMembershipPda,
        member: kp.publicKey,
        membership: membershipPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log(`Granted ${scenario.role} to ${scenario.name} (${kp.publicKey.toBase58()}).`);

    if (scenario.name === "h3-revoked") {
      await revokeH3Membership(orgProgram, {
        authority: payer.publicKey,
        organization: orgPda,
        adminMembership: adminMembershipPda,
        member: kp.publicKey,
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
