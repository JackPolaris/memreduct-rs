#!/usr/bin/env node
// Standardised release flow for Mem Reduct.
//
// Why this exists: the release used to be a hand-run sequence of six steps
// spread over README/CONTRIBUTING, and every one of them could go wrong
// silently — a forgotten `Cargo.lock` version, a hand-typed manifest URL that
// did not match the uploaded asset name, a missing `latest.json` (the Tauri CLI
// does not generate one), a signing key that does not match the embedded public
// key (discovered only after an 11-minute build), and a `git push` blocked by a
// filtering proxy. This script encodes all of it, fails fast *before* the long
// build, and verifies the published result afterwards.
//
// Usage (from the repo root):
//   node scripts/release.mjs preflight         # checks only, no side effects
//   node scripts/release.mjs bump 3.5.14       # sync the 5 version files
//   node scripts/release.mjs build             # signed NSIS build
//   node scripts/release.mjs manifest          # rename assets + write manifest
//   node scripts/release.mjs publish           # commit, tag, push, gh release
//   node scripts/release.mjs verify            # re-check the published release
//   node scripts/release.mjs all 3.5.14        # the whole thing, in order
//
// Options:
//   --bundles <nsis|msi|all>   default: nsis
//   --allow-dirty              skip the clean-tree requirement (preflight)
//   --dry-run                  print what would happen, change nothing
//   -y, --yes                  do not ask for confirmation before publishing

import { execFileSync } from "node:child_process";
import { createHash, createPublicKey, verify as edVerify } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline/promises";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const REPO = "JackPolaris/memreduct-rs";
const BRANCH = "master";
const BUNDLE_DIR = "src-tauri/target/release/bundle";
const KEY_FILE = path.join(os.homedir(), ".tauri", "memreduct.key");

// ---------------------------------------------------------------- utilities

const argv = process.argv.slice(2);
const flags = new Set(argv.filter((a) => a.startsWith("-")));
const positional = argv.filter((a) => !a.startsWith("-"));
const DRY = flags.has("--dry-run") || flags.has("-n");
const ASK = !flags.has("--yes") && !flags.has("-y");

function bundles() {
  const i = argv.indexOf("--bundles");
  const v = i >= 0 ? argv[i + 1] : "nsis";
  if (!["nsis", "msi", "all"].includes(v)) fail(`--bundles 只能是 nsis / msi / all，收到 ${v}`);
  return v;
}

const step = (name) => console.log(`\n\x1b[1m\x1b[36m== ${name}\x1b[0m`);
const dim = (msg) => console.log(`\x1b[2m   ${msg}\x1b[0m`);
const info = (msg) => console.log(`   ${msg}`);
const ok = (msg) => console.log(`   \x1b[32m✓\x1b[0m ${msg}`);
const warn = (msg) => console.log(`   \x1b[33m!\x1b[0m ${msg}`);
function fail(msg) {
  console.error(`\n\x1b[31m✗ ${msg}\x1b[0m`);
  process.exit(1);
}

function sh(cmd, args, opts = {}) {
  // stderr is captured rather than inherited: expected failures (e.g. a 404 when
  // probing whether a tag exists) would otherwise print noise into the middle of
  // the step output. `trySh` returns it so callers can still show it.
  return execFileSync(cmd, args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["pipe", "pipe", "pipe"],
    ...opts,
  });
}
function trySh(cmd, args, opts = {}) {
  try {
    return { ok: true, out: sh(cmd, args, opts) };
  } catch (e) {
    return { ok: false, out: `${e.stdout ?? ""}${e.stderr ?? e.message}` };
  }
}
const git = (...args) => sh("git", args).trim();
const gh = (args, input) => sh("gh", args, input ? { input } : {});
const readJson = (p) => JSON.parse(fs.readFileSync(path.join(ROOT, p), "utf8"));
const writeJson = (p, obj) =>
  fs.writeFileSync(path.join(ROOT, p), JSON.stringify(obj, null, 2) + "\n");

/** Rust architecture name, as the updater uses it for the platform key. */
function archName() {
  if (process.arch === "arm64") return "aarch64";
  if (process.arch === "ia32") return "i686";
  return "x86_64";
}
/** Release target triple, mirroring `src-tauri/src/updater.rs`. */
function targetTriple() {
  return `${archName()}-pc-windows-msvc`;
}
/** NSIS artifact architecture tag. */
function archTag() {
  if (process.arch === "arm64") return "arm64";
  if (process.arch === "ia32") return "x86";
  return "x64";
}
/** Platform key looked up by the updater plugin: `{os}-{arch}`. */
const platformKey = () => `windows-${archName()}`;
const manifestName = () => `update-${targetTriple()}.json`;

/** The five files that must agree on the version. */
function currentVersions() {
  const lock = readJson("package-lock.json");
  const cargoToml = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.toml"), "utf8");
  const cargoLock = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.lock"), "utf8");
  return {
    "package.json": readJson("package.json").version,
    "package-lock.json": lock.version,
    'package-lock.json [packages.""]': lock.packages?.[""]?.version,
    "src-tauri/Cargo.toml": cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
    "src-tauri/Cargo.lock": cargoLock.match(
      /\[\[package\]\]\r?\nname = "mem-reduct"\r?\nversion = "([^"]+)"/
    )?.[1],
    "src-tauri/tauri.conf.json": readJson("src-tauri/tauri.conf.json").version,
  };
}

/** Release notes for the manifest, taken from the CHANGELOG section. */
function changelogNotes(version) {
  const p = path.join(ROOT, "CHANGELOG.md");
  if (!fs.existsSync(p)) return "";
  const lines = fs.readFileSync(p, "utf8").split(/\r?\n/);
  const start = lines.findIndex((l) =>
    new RegExp(`^##\\s+v?${version.replace(/\./g, "\\.")}\\b`).test(l)
  );
  if (start < 0) return "";
  const out = [];
  for (let i = start + 1; i < lines.length; i++) {
    if (/^##\s/.test(lines[i])) break;
    const m = lines[i].match(/^\s*[-*]\s+(.+)$/);
    if (m) out.push(`- ${m[1].replace(/\*\*/g, "").trim()}`);
  }
  return out.slice(0, 6).join("\n");
}

/** A file inside the OS temp dir — never inside the repo, so `git add -A` cannot pick it up. */
const tempFile = (name) => path.join(os.tmpdir(), `memreduct-release-${name}`);

// ------------------------------------------- minisign verification, no deps

/**
 * Verify a Tauri/minisign signature over `file` using the public key embedded in
 * `tauri.conf.json`.
 *
 * minisign layout: a base64 blob that decodes to
 *   "untrusted comment: ...\n<base64 of alg(2) | keyid(8) | key-or-signature>"
 * The `ED` algorithm signs the BLAKE2b-512 digest of the file. Node ships both
 * Ed25519 and blake2b512, so no extra dependency is needed — and checking this
 * *before* publishing is what prevents a release every client would reject.
 */
function verifySignature(file, signatureB64, pubkeyB64) {
  const decode = (b64) => Buffer.from(Buffer.from(b64, "base64").toString("utf8").split("\n")[1], "base64");
  const pub = decode(pubkeyB64);
  const sig = decode(signatureB64);
  // minisign prints the key id in the reverse of its on-disk byte order, so
  // reverse it here to match the id shown in the `.pub` comment (and in
  // `tauri.conf.json`), which is what a human compares against.
  const keyId = Buffer.from(pub.subarray(2, 10)).reverse().toString("hex").toUpperCase();
  const sigKeyId = Buffer.from(sig.subarray(2, 10)).reverse().toString("hex").toUpperCase();
  const alg = sig.subarray(0, 2).toString("latin1");

  const data = fs.readFileSync(file);
  const payload =
    alg === "ED" ? createHash("blake2b512").update(data).digest() : alg === "Ed" ? data : null;
  if (!payload) throw new Error(`未知的 minisign 算法: ${JSON.stringify(alg)}`);
  if (keyId !== sigKeyId) throw new Error(`key id 不一致: 公钥 ${keyId} / 签名 ${sigKeyId}`);

  // Raw 32-byte Ed25519 key -> SPKI DER (fixed 12-byte prefix).
  const keyObj = createPublicKey({
    key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), pub.subarray(10)]),
    format: "der",
    type: "spki",
  });
  if (!edVerify(null, payload, keyObj, sig.subarray(10))) throw new Error("Ed25519 验签失败");
  return { keyId, alg, mode: alg === "ED" ? "BLAKE2b-512 预哈希" : "原始数据" };
}

// -------------------------------------------------------------- preflight

/**
 * Sanity checks, all of which are cheap compared with the build that follows.
 *
 * `target` is the version about to be released. It differs from the version in
 * the files when this runs as part of `all` (before `bump`), and the tag check
 * must look at the *target* — checking the current version would always fail,
 * because that version has just been released.
 */
function cmdPreflight(target) {
  step("preflight");
  if (target && !/^\d+\.\d+\.\d+$/.test(target)) fail(`版本号必须是 x.y.z,收到 ${target}`);

  const dirty = git("status", "--porcelain");
  if (dirty) {
    if (flags.has("--allow-dirty")) {
      warn(`工作区有未提交改动(--allow-dirty 已忽略):\n${dirty}`);
    } else {
      fail(
        `工作区不干净,发版提交会把无关改动一起带上:\n${dirty}\n先提交或 stash,或显式加 --allow-dirty。`
      );
    }
  } else ok("工作区干净");

  const branch = git("rev-parse", "--abbrev-ref", "HEAD");
  if (branch !== BRANCH) warn(`当前分支是 ${branch},发布流程默认走 ${BRANCH}`);
  else ok(`分支 ${branch}`);

  const versions = currentVersions();
  const unique = [...new Set(Object.values(versions))];
  if (unique.length !== 1 || !unique[0]) {
    fail(
      `版本号不一致(跑 \`bump <x.y.z>\` 修):\n` +
        Object.entries(versions)
          .map(([k, v]) => `   ${k} = ${v ?? "<未找到>"}`)
          .join("\n")
    );
  }
  const current = unique[0];
  ok(`版本号一致: ${current}(${Object.keys(versions).length} 处)`);

  const release = target ?? current;
  if (target) {
    if (compareVersions(target, current) <= 0) {
      fail(`目标版本 ${target} 没有高于当前版本 ${current}`);
    }
    ok(`目标版本 ${target}(当前 ${current},将同步 bump)`);
    if (!changelogNotes(target)) {
      warn(`CHANGELOG 里没有 "## v${target}" 小节 —— manifest 的 notes 会退化成只有版本号`);
    } else {
      ok(`CHANGELOG 已有 v${target} 小节,notes 可用`);
    }
  }

  // Signing key present AND paired with the embedded public key. Getting this
  // wrong is only discovered after a full build, so it is checked up front.
  if (!fs.existsSync(KEY_FILE)) fail(`找不到签名私钥: ${KEY_FILE}`);
  const pubFile = `${KEY_FILE}.pub`;
  if (!fs.existsSync(pubFile)) fail(`找不到公钥文件: ${pubFile}`);
  const conf = readJson("src-tauri/tauri.conf.json");
  const pubkey = conf.plugins?.updater?.pubkey;
  if (!pubkey) fail("tauri.conf.json 缺少 plugins.updater.pubkey");
  if (fs.readFileSync(pubFile, "utf8").trim() !== pubkey.trim()) {
    fail(
      "签名私钥与 tauri.conf.json 的 pubkey 不配对!\n" +
        "   用它签名会让所有已安装版本拒绝更新,先修好再发版。"
    );
  }
  ok(`签名密钥与内置公钥配对(与 ${path.basename(pubFile)} 一致)`);

  if (!createHash("blake2b512")) fail("当前 Node 不支持 blake2b512,无法验签");
  ok(`Node ${process.version}(支持 blake2b512 + Ed25519)`);

  const auth = trySh("gh", ["auth", "status"]);
  if (!auth.ok) fail(`gh 未登录或无权限:\n${auth.out}`);
  ok("gh 已登录");

  if (trySh("git", ["rev-parse", "-q", "--verify", `refs/tags/v${release}`]).ok) {
    fail(`本地已存在 tag v${release}`);
  }
  const remoteTag = trySh("gh", ["api", `repos/${REPO}/git/ref/tags/v${release}`, "--jq", ".object.sha"]);
  if (remoteTag.ok && remoteTag.out.trim()) fail(`远端已存在 tag v${release}`);
  ok(`tag v${release} 未被占用`);

  ok("preflight 通过");
}

/** Numeric semver comparison (enough for `x.y.z`). */
function compareVersions(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (pa[i] !== pb[i]) return pa[i] - pb[i];
  return 0;
}

// ------------------------------------------------------------------- bump

function cmdBump(version) {
  if (!version) fail("用法: node scripts/release.mjs bump <x.y.z>");
  if (!/^\d+\.\d+\.\d+$/.test(version)) fail(`版本号必须是 x.y.z,收到 ${version}`);

  step(`bump -> ${version}`);
  const old = [...new Set(Object.values(currentVersions()))][0];
  const patch = (rel, mutate) => {
    const abs = path.join(ROOT, rel);
    const before = fs.readFileSync(abs, "utf8");
    const after = mutate(before);
    if (DRY) {
      info(`[dry-run] ${rel}: ${before === after ? "无变化!" : "将更新"}`);
      return;
    }
    if (before === after) fail(`${rel} 没有发生变化 —— 版本号可能已经是 ${version}`);
    fs.writeFileSync(abs, after);
  };

  // JSON files go through a parse/serialise round-trip: a `/version/` regex would
  // also hit every dependency entry in package-lock.json.
  patch("package.json", (s) => s.replace(/("version":\s*)"[^"]+"/, `$1"${version}"`));
  patch("package-lock.json", (s) => {
    const o = JSON.parse(s);
    o.version = version;
    if (o.packages?.[""]) o.packages[""].version = version;
    return JSON.stringify(o, null, 2) + "\n";
  });
  patch("src-tauri/tauri.conf.json", (s) => s.replace(/("version":\s*)"[^"]+"/, `$1"${version}"`));
  patch("src-tauri/Cargo.toml", (s) => s.replace(/^version = "[^"]+"/m, `version = "${version}"`));
  patch("src-tauri/Cargo.lock", (s) =>
    s.replace(
      /(\[\[package\]\]\r?\nname = "mem-reduct"\r?\nversion = ")[^"]+(")/,
      `$1${version}$2`
    )
  );

  const cl = path.join(ROOT, "CHANGELOG.md");
  const md = fs.readFileSync(cl, "utf8");
  const heading = new RegExp(`^##\\s+v?${version.replace(/\./g, "\\.")}[^\\n]*$`, "m");
  if (heading.test(md)) {
    if (!DRY) fs.writeFileSync(cl, md.replace(heading, `## v${version} (${new Date().toISOString().slice(0, 10)})`));
    info(`CHANGELOG: 标记 v${version} 为已发布`);
  } else {
    warn(`CHANGELOG 里没有 "## v${version}" 小节 —— manifest 的 notes 会退化成只有版本号`);
  }

  if (!DRY) {
    for (const [k, v] of Object.entries(currentVersions())) {
      if (v !== version) fail(`同步失败:${k} 仍是 ${v}`);
    }
  }
  ok(`${old} -> ${version},5 个文件全部同步`);
  info(`改动:`);
  for (const line of git("diff", "--stat").split("\n").filter(Boolean)) info(`  ${line}`);
}

// ------------------------------------------------------------------ build

function cmdBuild() {
  step("build");
  if (!fs.existsSync(KEY_FILE)) fail(`找不到签名私钥: ${KEY_FILE}`);
  const key = fs.readFileSync(KEY_FILE, "utf8").trim();

  // Both variables matter:
  //  - the key must be passed explicitly, because the release profile has
  //    `createUpdaterArtifacts: true` and a keyless build fails;
  //  - the password must be set *even when empty*: without the variable the CLI
  //    prompts for one, and in a non-interactive shell that hangs forever.
  const env = {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: key,
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
  };
  const args = ["tauri", "build", "--bundles", bundles()];
  info(`npx ${args.join(" ")}   (约 3–12 分钟;LTO 链接阶段没有输出属正常)`);
  if (DRY) {
    info("[dry-run] 跳过实际构建");
    return;
  }
  // stdio inherited for live progress; stdin closed so a stray prompt fails
  // immediately instead of hanging.
  execFileSync("npx", args, { cwd: ROOT, env, stdio: ["ignore", "inherit", "inherit"] });

  const version = readJson("package.json").version;
  const exe = path.join(ROOT, BUNDLE_DIR, "nsis", `Mem Reduct_${version}_${archTag()}-setup.exe`);
  if (!fs.existsSync(exe)) fail(`构建结束但没有找到安装包: ${exe}`);
  if (!fs.existsSync(`${exe}.sig`)) fail(`安装包没有签名 —— 签名密钥可能有问题: ${exe}`);
  ok(`安装包 + 签名已生成(${(fs.statSync(exe).size / 1024 / 1024).toFixed(2)} MB)`);
}

// --------------------------------------------------------------- manifest

function cmdManifest() {
  step("manifest");
  const version = readJson("package.json").version;
  const nsis = path.join(ROOT, BUNDLE_DIR, "nsis");
  const asset = `Mem.Reduct_${version}_${archTag()}-setup.exe`;
  const srcExe = path.join(nsis, `Mem Reduct_${version}_${archTag()}-setup.exe`);
  const exe = path.join(nsis, asset);
  const sig = `${exe}.sig`;

  if (!fs.existsSync(srcExe) && !fs.existsSync(exe)) {
    fail(`缺少构建产物,先跑 build:\n   ${srcExe}`);
  }
  if (DRY) {
    info(`[dry-run] 将把 "Mem Reduct_${version}_${archTag()}-setup.exe" 改名为 ${asset}`);
    info(`[dry-run] 将写入 ${manifestName()}(bundle 目录 + 仓库根)`);
    return;
  }

  // Spaces become dots: this name ends up in the download URL.
  if (fs.existsSync(srcExe)) fs.renameSync(srcExe, exe);
  if (fs.existsSync(`${srcExe}.sig`)) fs.renameSync(`${srcExe}.sig`, sig);
  info(`资产名: ${asset}`);

  const signature = fs.readFileSync(sig, "utf8").trim();
  const conf = readJson("src-tauri/tauri.conf.json");

  // Refuse to publish a signature the clients would reject.
  let checked;
  try {
    checked = verifySignature(exe, signature, conf.plugins.updater.pubkey);
  } catch (e) {
    fail(`签名验证失败,不能发布:${e.message}`);
  }
  ok(`签名验证通过(${checked.mode},key id ${checked.keyId})`);

  const notes = changelogNotes(version);
  const manifest = {
    version,
    notes: notes ? `Mem Reduct ${version}\n\n${notes}` : `Mem Reduct ${version}`,
    pub_date: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
    platforms: {
      [platformKey()]: {
        signature,
        url: `https://github.com/${REPO}/releases/download/v${version}/${asset}`,
      },
    },
  };

  writeJson(path.join(BUNDLE_DIR, "nsis", manifestName()), manifest);
  // Repo copy: the reference for the manifest format, tracked in git.
  writeJson(manifestName(), manifest);
  ok(`更新清单已写入 bundle/nsis/${manifestName()} 与仓库根 ${manifestName()}`);
  ok(`平台键 ${platformKey()},notes ${notes ? `${notes.split("\n").length} 条要点(来自 CHANGELOG)` : "无(只有版本号)"}`);
}

// ---------------------------------------------------------------- publish

async function cmdPublish() {
  step("publish");
  const version = readJson("package.json").version;
  const tag = `v${version}`;
  const nsis = path.join(ROOT, BUNDLE_DIR, "nsis");
  const asset = `Mem.Reduct_${version}_${archTag()}-setup.exe`;
  const files = [
    path.join(nsis, asset),
    path.join(nsis, `${asset}.sig`),
    path.join(nsis, manifestName()),
  ];
  for (const f of files) if (!fs.existsSync(f)) fail(`缺少发布资产: ${f}`);

  const notes = changelogNotes(version);
  const notesFile = tempFile("notes.md");
  fs.writeFileSync(notesFile, `## Mem Reduct ${version}\n\n${notes}\n`);
  const msgFile = tempFile("commit-msg.txt");
  fs.writeFileSync(msgFile, `release: ${tag}\n\n${notes}\n`);

  console.log(`\n将发布 ${tag}:`);
  for (const f of files) console.log(`   ${path.basename(f)}`);
  console.log(`   提交当前改动 -> 打 tag -> 推送 -> gh release create`);

  if (ASK && !DRY) {
    if (process.stdin.isTTY) {
      const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
      const answer = await rl.question("\n继续? [y/N] ");
      rl.close();
      if (!/^y(es)?$/i.test(answer.trim())) {
        fs.rmSync(notesFile, { force: true });
        fs.rmSync(msgFile, { force: true });
        fail("已取消(未做任何改动)");
      }
    } else {
      warn("非交互环境,跳过确认(加 --yes 可消除本提示)");
    }
  }

  if (DRY) {
    info(`[dry-run] git add -A && git commit -F <tmp> && git tag -a ${tag}`);
    info(`[dry-run] git push origin ${BRANCH} && git push origin ${tag}`);
    info(`[dry-run] gh release create ${tag} --notes-file <tmp> <3 assets>`);
    fs.rmSync(notesFile, { force: true });
    fs.rmSync(msgFile, { force: true });
    return;
  }

  git("add", "-A");
  // Idempotent: the publish step may be re-run after a partial failure (e.g. the
  // push failing), by which time everything is already committed.
  if (git("status", "--porcelain")) {
    git("commit", "-F", msgFile);
    ok(`已提交: ${git("log", "-1", "--oneline")}`);
  } else {
    ok(`没有待提交改动,复用 HEAD: ${git("log", "-1", "--oneline")}`);
  }
  fs.rmSync(msgFile, { force: true });

  if (trySh("git", ["rev-parse", "-q", "--verify", `refs/tags/${tag}`]).ok) {
    ok(`tag ${tag} 已存在,跳过`);
  } else {
    git("tag", "-a", tag, "-m", `Mem Reduct ${version}`);
    ok(`已打 tag ${tag}`);
  }

  const pushBranch = trySh("git", ["push", "origin", BRANCH]);
  const pushTag = pushBranch.ok ? trySh("git", ["push", "origin", tag]) : { ok: false, out: "" };
  if (pushBranch.ok && pushTag.ok) {
    ok(`已推送 ${BRANCH} 与 ${tag}`);
  } else {
    warn("git push 失败(常见原因:代理过滤 github.com),改用 GitHub REST API 推送");
    const lastLine = String(pushBranch.out).trim().split("\n").pop();
    if (lastLine) dim(lastLine);
    pushViaApi(tag);
  }

  gh([
    "release", "create", tag,
    "--title", `Mem Reduct ${version}`,
    "--notes-file", notesFile,
    ...files.map((f) => path.relative(ROOT, f)),
  ]);
  fs.rmSync(notesFile, { force: true });
  ok(`已创建 release: https://github.com/${REPO}/releases/tag/${tag}`);

  cmdVerify();
}

/**
 * Push the current branch and tag through the Git Data API.
 *
 * Used when `github.com` is unreachable — a proxy that filters it answers 502
 * while api.github.com stays reachable. Objects are rebuilt from the local
 * repository and every created commit SHA is asserted against the local one, so
 * the reproduction is exact and the two sides cannot diverge. Ref moves happen
 * only after all objects exist.
 */
function pushViaApi(tag) {
  const api = (endpoint, method, body) =>
    JSON.parse(gh(["api", endpoint, "-X", method].concat(body ? ["--input", "-"] : []),
      body ? JSON.stringify(body) : undefined) || "null");
  const jq = (endpoint, expr) => gh(["api", endpoint, "--jq", expr]).trim();

  const remoteHead = jq(`repos/${REPO}/git/ref/heads/${BRANCH}`, ".object.sha");
  info(`远端 ${BRANCH} = ${remoteHead.slice(0, 7)}`);

  const pending = git("rev-list", "--reverse", `${remoteHead}..HEAD`).split(/\s+/).filter(Boolean);
  let parent = remoteHead;
  let baseTree = jq(`repos/${REPO}/git/commits/${remoteHead}`, ".tree.sha");

  for (const sha of pending) {
    const raw = git("cat-file", "commit", sha);
    // Split on the FIRST blank line only: a commit message of the form
    // "subject\n\nbody" — which is the normal shape — would otherwise be cut
    // apart by a naive split(/\n\n/), truncating the message and producing a
    // different SHA than the local commit.
    const splitAt = raw.indexOf("\n\n");
    const header = raw.slice(0, splitAt);
    const message = raw.slice(splitAt + 2);
    const meta = (key) => {
      const line = header.split("\n").find((l) => l.startsWith(`${key} `));
      const m = line.slice(key.length + 1).match(/^(.*) <(.*)> (\d+) ([+-]\d{4})$/);
      if (!m) fail(`无法解析 ${key} 行: ${line}`);
      const offMin = (m[4][0] === "-" ? -1 : 1) * (Number(m[4].slice(1, 3)) * 60 + Number(m[4].slice(3)));
      const iso = new Date((Number(m[3]) + offMin * 60) * 1000)
        .toISOString()
        .replace("Z", `${m[4].slice(0, 3)}:${m[4].slice(3)}`);
      return { name: m[1], email: m[2], date: iso };
    };

    const tree = [];
    for (const line of git("diff-tree", "-r", "--no-commit-id", "--name-status", parent, sha)
      .split("\n")
      .filter(Boolean)) {
      const [status, p] = line.split("\t");
      if (status.startsWith("D")) {
        tree.push({ path: p, mode: "100644", type: "blob", sha: null });
        continue;
      }
      const content = execFileSync("git", ["show", `${sha}:${p}`], { cwd: ROOT, maxBuffer: 1 << 28 });
      const blob = api(`/repos/${REPO}/git/blobs`, "POST", {
        content: content.toString("base64"),
        encoding: "base64",
      });
      tree.push({ path: p, mode: git("ls-tree", sha, "--", p).split(/\s+/)[0], type: "blob", sha: blob.sha });
    }

    const newTree = api(`/repos/${REPO}/git/trees`, "POST", { base_tree: baseTree, tree });
    const commit = api(`/repos/${REPO}/git/commits`, "POST", {
      message,
      tree: newTree.sha,
      parents: [parent],
      author: meta("author"),
      committer: meta("committer"),
    });
    if (commit.sha !== sha) {
      fail(`复现的提交 SHA 不一致(本地 ${sha} / 远端 ${commit.sha}),已中止,远端 ref 未改动`);
    }
    info(`${sha.slice(0, 7)} -> 已重建,SHA 与本地一致`);
    parent = commit.sha;
    baseTree = jq(`repos/${REPO}/git/commits/${parent}`, ".tree.sha");
  }

  if (pending.length) {
    api(`/repos/${REPO}/git/refs/heads/${BRANCH}`, "PATCH", { sha: parent, force: false });
    ok(`已通过 API 更新 ${BRANCH} -> ${parent.slice(0, 7)}`);
  }

  const existing = trySh("gh", ["api", `repos/${REPO}/git/ref/tags/${tag}`, "--jq", ".object.sha"]);
  if (!(existing.ok && existing.out.trim())) {
    // Tag the commit the *local* annotated tag points at, not the branch head:
    // the branch may carry commits published after the release commit, and the
    // two must agree or the tag would name the wrong tree.
    const localTarget = trySh("git", ["rev-list", "-n", "1", tag]);
    const target =
      localTarget.ok && localTarget.out.trim()
        ? localTarget.out.trim()
        : jq(`repos/${REPO}/git/ref/heads/${BRANCH}`, ".object.sha");
    const tagObj = api(`/repos/${REPO}/git/tags`, "POST", {
      tag,
      message: `Mem Reduct ${version}`,
      object: target,
      type: "commit",
    });
    api(`/repos/${REPO}/git/refs`, "POST", { ref: `refs/tags/${tag}`, sha: tagObj.sha });
    ok(`已通过 API 创建 tag ${tag} -> ${target.slice(0, 7)}`);
  }
}

// ----------------------------------------------------------------- verify

function cmdVerify() {
  step("verify");
  const version = readJson("package.json").version;
  const tag = `v${version}`;
  const nsis = path.join(ROOT, BUNDLE_DIR, "nsis");
  const sha256 = (f) => createHash("sha256").update(fs.readFileSync(f)).digest("hex");

  const remoteHead = gh(["api", `repos/${REPO}/git/ref/heads/${BRANCH}`, "--jq", ".object.sha"]).trim();
  const localHead = git("rev-parse", "HEAD");
  if (remoteHead === localHead) ok(`分支一致 ${localHead.slice(0, 7)}`);
  else warn(`远端 ${remoteHead.slice(0, 7)} != 本地 ${localHead.slice(0, 7)}`);

  const rel = trySh("gh", ["api", `repos/${REPO}/releases/tags/${tag}`]);
  if (!rel.ok) fail(`远端没有 release ${tag}`);
  const release = JSON.parse(rel.out);
  if (release.draft || release.prerelease) warn(`release ${tag} 是 draft/prerelease,客户端看不到`);
  for (const a of release.assets) {
    const local = path.join(nsis, a.name);
    let digest = "";
    if (fs.existsSync(local) && a.digest) {
      digest = a.digest.endsWith(sha256(local)) ? "  sha256 一致 ✓" : "  sha256 不一致 ✗";
      if (!a.digest.endsWith(sha256(local))) fail(`${a.name} 的上传内容与本地不一致`);
    }
    ok(`${a.state === "uploaded" ? "已上传" : a.state} ${a.name} (${a.size} 字节)${digest}`);
  }

  const expected = [`Mem.Reduct_${version}_${archTag()}-setup.exe`, `Mem.Reduct_${version}_${archTag()}-setup.exe.sig`, manifestName()];
  for (const name of expected) {
    if (!release.assets.some((a) => a.name === name)) fail(`release 缺少资产: ${name}`);
  }
  ok("三个资产齐全");

  const latest = gh(["api", `repos/${REPO}/releases/latest`, "--jq", ".tag_name"]).trim();
  if (latest === tag) ok(`releases/latest -> ${tag}(客户端更新入口)`);
  else fail(`releases/latest 指向 ${latest},不是 ${tag}`);

  const assetId = release.assets.find((a) => a.name === manifestName()).id;
  const remote = JSON.parse(
    gh(["api", `repos/${REPO}/releases/assets/${assetId}`, "-H", "Accept: application/octet-stream"])
  );
  if (remote.version !== version) fail(`远端清单版本是 ${remote.version},期望 ${version}`);
  const entry = remote.platforms?.[platformKey()];
  if (!entry) fail(`远端清单缺少平台键 ${platformKey()}(实际 ${Object.keys(remote.platforms ?? {})})`);
  if (entry.url.split("/").pop() !== expected[0]) fail(`清单 url 与资产命名不符: ${entry.url}`);
  ok(`远端清单版本 ${remote.version}、平台键 ${platformKey()}、url 文件名字段正确`);

  const conf = readJson("src-tauri/tauri.conf.json");
  const checked = verifySignature(path.join(nsis, expected[0]), entry.signature, conf.plugins.updater.pubkey);
  ok(`远端清单签名可用内置公钥验证(${checked.mode},key id ${checked.keyId})`);

  ok(`${tag} 发布完成`);
}

// ------------------------------------------------------------------- main

const version = readJson("package.json").version;
const COMMANDS = {
  preflight: () => cmdPreflight(positional[1]),
  bump: () => cmdBump(positional[1]),
  build: () => cmdBuild(),
  manifest: () => cmdManifest(),
  publish: () => cmdPublish(),
  verify: () => cmdVerify(),
  all: async () => {
    const target = positional[1];
    if (!target) fail("用法: node scripts/release.mjs all <x.y.z>");
    // Preflight validates the *target* version: the current one has just been
    // released, so its tag necessarily exists.
    cmdPreflight(target);
    cmdBump(target);
    cmdBuild();
    cmdManifest();
    await cmdPublish();
  },
};

const cmd = positional[0];
if (!cmd || !COMMANDS[cmd]) {
  console.log(
    "用法: node scripts/release.mjs <preflight [x.y.z]|bump <x.y.z>|build|manifest|publish|verify|all <x.y.z>>\n" +
      "选项: --bundles <nsis|msi|all>  --dry-run  --yes  --allow-dirty"
  );
  process.exit(cmd ? 1 : 0);
}
console.log(`\x1b[2m仓库 ${ROOT}\n目标 ${targetTriple()} / 资产后缀 ${archTag()} / 当前版本 ${version}${DRY ? "   [dry-run]" : ""}\x1b[0m`);
await COMMANDS[cmd]();
