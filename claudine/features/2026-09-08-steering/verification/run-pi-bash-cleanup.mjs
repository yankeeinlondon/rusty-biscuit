#!/usr/bin/env node

import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const piBinary = process.env.CLAUDINE_PI_BINARY;
if (!piBinary) throw new Error("CLAUDINE_PI_BINARY must name the Pi binary under test");
if (process.platform === "win32") throw new Error("This POSIX process-group experiment does not support Windows");
const here = dirname(fileURLToPath(import.meta.url));
const extension = join(here, "pi-bash-cleanup-probe.ts");
const reportPath = process.env.CLAUDINE_PI_CLEANUP_REPORT;
if (!reportPath) throw new Error("CLAUDINE_PI_CLEANUP_REPORT must name the evidence output file");
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function alive(pid) {
	try { process.kill(pid, 0); return true; } catch { return false; }
}

async function processIdentity(pid) {
	return await new Promise((resolve) => {
		const child = spawn("/bin/ps", ["-p", String(pid), "-o", "pgid=", "-o", "command="], { stdio: ["ignore", "pipe", "ignore"] });
		let output = "";
		child.stdout.on("data", (chunk) => { output += chunk; });
		child.once("exit", (code) => resolve(code === 0 ? output.trim() : ""));
	});
}

async function waitFor(test, description, timeoutMs = 10_000) {
	const deadline = Date.now() + timeoutMs;
	while (!(await test())) {
		if (Date.now() >= deadline) throw new Error(`Timed out waiting for ${description}`);
		await delay(20);
	}
}

async function runScenario(name, terminate) {
	const root = await mkdtemp(join(tmpdir(), "claudine-pi-cleanup-"));
	const cleanupDir = join(root, "probe");
	const agentDir = join(root, "agent");
	const sessions = join(root, "sessions");
	const marker = `claudine-pi-cleanup-${process.pid}-${name}`;
	await Promise.all([
		mkdir(cleanupDir, { recursive: true }),
		mkdir(agentDir, { recursive: true }),
		mkdir(sessions, { recursive: true }),
		mkdir(join(root, ".pi", "skills", "cleanup-probe-skill"), { recursive: true }),
		mkdir(join(root, ".pi", "prompts"), { recursive: true }),
	]);
	await Promise.all([
		writeFile(join(root, "AGENTS.md"), "CLEANUP_CONTEXT_NONCE\n"),
		writeFile(join(root, ".pi", "skills", "cleanup-probe-skill", "SKILL.md"), "---\nname: cleanup-probe-skill\ndescription: deterministic cleanup probe\n---\nfixture\n"),
		writeFile(join(root, ".pi", "prompts", "cleanup-probe-template.md"), "---\ndescription: deterministic cleanup probe\n---\nfixture\n"),
	]);

	const child = spawn(piBinary, ["--mode", "rpc", "--offline", "--approve", "--provider", "claudine-bash-cleanup-probe", "--model", "fixture", "--thinking", "off", "--extension", extension, "--session-dir", sessions], {
		cwd: root,
		env: { ...process.env, PI_CODING_AGENT_DIR: agentDir, PI_TELEMETRY: "0", CLAUDINE_PI_CLEANUP_DIR: cleanupDir, CLAUDINE_PI_CLEANUP_MARKER: marker },
		stdio: ["pipe", "pipe", "pipe"],
	});
	let stdout = "";
	child.stdout.on("data", (chunk) => { stdout += chunk; });
	child.stderr.resume();
	const exited = new Promise((resolve) => child.once("exit", (code, signal) => resolve({ code, signal })));
	const events = () => stdout.split("\n").filter(Boolean).flatMap((line) => {
		try { return [JSON.parse(line)]; } catch { return []; }
	});
	let shellPid;
	let fixtureCleanupRequired = false;
	try {
		child.stdin.write(`${JSON.stringify({ id: "commands", type: "get_commands" })}\n`);
		await waitFor(() => events().some((event) => event.type === "response" && event.id === "commands"), "command discovery");
		const commands = events().find((event) => event.type === "response" && event.id === "commands")?.data?.commands ?? [];
		child.stdin.write(`${JSON.stringify({ id: "prompt", type: "prompt", message: "RUN_CLEANUP_PROBE" })}\n`);
		await waitFor(async () => {
			try {
				shellPid = Number.parseInt(await readFile(join(cleanupDir, "shell.pid"), "utf8"), 10);
				return Number.isInteger(shellPid);
			} catch { return false; }
		}, "built-in bash subprocess");
		const shellIdentity = await processIdentity(shellPid);
		if (!shellIdentity.startsWith(String(shellPid)) || !shellIdentity.includes(marker)) {
			throw new Error("fixture process identity did not match its private process group and marker");
		}
		await terminate(child, events);
		await delay(750);
		const shellAliveBeforeFixtureCleanup = alive(shellPid);
		fixtureCleanupRequired = shellAliveBeforeFixtureCleanup;
		if (fixtureCleanupRequired) {
			try { process.kill(-shellPid, "SIGKILL"); } catch { try { process.kill(shellPid, "SIGKILL"); } catch {} }
		}
		await waitFor(() => !alive(shellPid), "fixture cleanup", 5_000);
		const resources = JSON.parse(await readFile(join(cleanupDir, "resources.json"), "utf8"));
		resources.skill_command = commands.some((command) => command.source === "skill" && command.name === "skill:cleanup-probe-skill");
		resources.template_command = commands.some((command) => command.source === "prompt" && command.name === "cleanup-probe-template");
		return {
			scenario: name,
			pi_exit: await Promise.race([exited, delay(2_000).then(() => null)]),
			observation_window_ms: 750,
			provider_cleanup_observed: !shellAliveBeforeFixtureCleanup,
			shell_alive_before_fixture_cleanup: shellAliveBeforeFixtureCleanup,
			fixture_cleanup_required: fixtureCleanupRequired,
			shell_alive_after_fixture_cleanup: alive(shellPid),
			resource_discovery: resources,
		};
	} finally {
		if (shellPid && alive(shellPid)) {
			try { process.kill(-shellPid, "SIGKILL"); } catch { try { process.kill(shellPid, "SIGKILL"); } catch {} }
		}
		if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
		await Promise.race([exited, delay(2_000)]);
		await rm(root, { recursive: true, force: true });
	}
}

const versionResult = await new Promise((resolve, reject) => {
	const child = spawn(piBinary, ["--version"], { stdio: ["ignore", "pipe", "pipe"] });
	let value = "";
	const timer = setTimeout(() => { child.kill("SIGKILL"); reject(new Error("pi --version timed out")); }, 5_000);
	child.stdout.on("data", (chunk) => { value += chunk; });
	child.once("error", (error) => { clearTimeout(timer); reject(error); });
	child.once("exit", (code) => { clearTimeout(timer); code === 0 ? resolve(value.trim()) : reject(new Error(`pi --version exited ${code}`)); });
});

const abort = await runScenario("rpc_abort", async (child, events) => {
	child.stdin.write(`${JSON.stringify({ id: "abort", type: "abort" })}\n`);
	await waitFor(() => events().some((line) => line.type === "response" && line.id === "abort" && line.success === true), "abort response");
});

const abruptKill = await runScenario("pi_sigkill", async (child) => {
	child.kill("SIGKILL");
});

if (!abort.provider_cleanup_observed || abort.fixture_cleanup_required) {
	throw new Error("RPC abort did not clean up the marked built-in bash subprocess");
}
if (abruptKill.provider_cleanup_observed || !abruptKill.fixture_cleanup_required) {
	throw new Error("SIGKILL unexpectedly ran Pi provider cleanup");
}
if ([abort, abruptKill].some((result) => result.shell_alive_after_fixture_cleanup)) {
	throw new Error("fixture cleanup left a subprocess alive");
}

const report = {
	provider: "pi",
	provider_version: versionResult,
	tested_on: "2026-09-08",
	os: process.platform === "darwin" ? "macos" : process.platform,
	fixture: "real Pi RPC with deterministic local model and built-in bash tool",
	production_activation: false,
	results: [abort, abruptKill],
	limitations: [
		"Executed on macOS only; source inspection indicates separate taskkill.exe behavior on Windows.",
		"SIGKILL intentionally prevents Pi shutdown handlers; the fixture performs and records any required cleanup afterward.",
		"Provider cleanup was observed after an exact 750 ms window by checking the marked exec-replaced shell process; additional descendants were not tested.",
		"The command is self-bounded to 20 seconds even if both provider and fixture cleanup fail.",
	],
};
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
process.stdout.write(`${reportPath}\n`);
