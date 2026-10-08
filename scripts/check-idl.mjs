import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
let drift = false;
for (const name of ["organization", "film"]) {
  const built = JSON.parse(readFileSync(join(root, "target", "idl", `${name}.json`), "utf8"));
  const committed = JSON.parse(readFileSync(join(root, "idl", `${name}.json`), "utf8"));
  if (!isDeepStrictEqual(built, committed)) {
    console.error(`IDL drift: idl/${name}.json differs from target/idl/${name}.json`);
    drift = true;
  }
}
if (drift) {
  console.error("Run `anchor build` and copy target/idl/*.json to idl/, then regenerate the clients.");
  process.exit(1);
}
console.log("IDL up to date");
