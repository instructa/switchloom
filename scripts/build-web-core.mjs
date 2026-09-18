import { spawnSync } from "node:child_process";
const run = (command, args) => {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
};
run("cargo", ["build", "--locked", "--release", "--lib", "--target", "wasm32-unknown-unknown"]);
run("wasm-bindgen", ["target/wasm32-unknown-unknown/release/model_routing.wasm", "--target", "web", "--out-dir", "target/web", "--out-name", "routing"]);
