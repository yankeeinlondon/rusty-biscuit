import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import {
	type Api,
	type AssistantMessage,
	type AssistantMessageEventStream,
	type Context,
	type Model,
	type SimpleStreamOptions,
	createAssistantMessageEventStream,
} from "@earendil-works/pi-ai";
import { Type } from "typebox";

const PROVIDER = "claudine-probe";
const MODEL = "fixture";
const PROBE_TIMEOUT_MS = 30_000;
const POLL_INTERVAL_MS = 20;

function probeDirectory(): string {
	const directory = process.env.CLAUDINE_PI_PROBE_DIR;
	if (!directory) throw new Error("CLAUDINE_PI_PROBE_DIR is required");
	return directory;
}

async function mark(name: string, contents = "ready\n"): Promise<void> {
	const directory = probeDirectory();
	await mkdir(directory, { recursive: true });
	await writeFile(join(directory, name), contents, "utf8");
}

function abortError(): Error {
	const error = new Error("Pi probe tool aborted");
	error.name = "AbortError";
	return error;
}

async function waitForRelease(slot: string, signal: AbortSignal): Promise<void> {
	const release = join(probeDirectory(), `release-${slot}`);
	const deadline = Date.now() + PROBE_TIMEOUT_MS;
	while (!existsSync(release)) {
		if (signal.aborted) throw abortError();
		if (Date.now() >= deadline) throw new Error(`Timed out waiting for release-${slot}`);
		await new Promise<void>((resolve, reject) => {
			const timer = setTimeout(resolve, POLL_INTERVAL_MS);
			const onAbort = () => {
				clearTimeout(timer);
				reject(abortError());
			};
			signal.addEventListener("abort", onAbort, { once: true });
			setTimeout(() => signal.removeEventListener("abort", onAbort), POLL_INTERVAL_MS + 1);
		});
	}
}

function userText(message: Context["messages"][number]): string | undefined {
	if (message.role !== "user") return undefined;
	if (typeof message.content === "string") return message.content;
	return message.content
		.filter((part) => part.type === "text")
		.map((part) => part.text)
		.join("");
}

function projectFixtureText(text: string): string {
	for (const marker of ["PROBE_TOOL_BATCH", "STEERING_NONCE", "TEMPLATE_NONCE", "SKILL_NONCE"]) {
		if (text.includes(marker)) return marker;
	}
	return "[non-fixture input]";
}

function newMessage(model: Model<Api>): AssistantMessage {
	return {
		role: "assistant",
		content: [],
		api: model.api,
		provider: model.provider,
		model: model.id,
		usage: {
			input: 0,
			output: 0,
			cacheRead: 0,
			cacheWrite: 0,
			totalTokens: 0,
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
		},
		stopReason: "pending",
		timestamp: Date.now(),
	};
}

function pushToolCall(
	stream: AssistantMessageEventStream,
	output: AssistantMessage,
	id: string,
	slot: string,
): void {
	const contentIndex = output.content.length;
	const toolCall = { type: "toolCall" as const, id, name: "probe_wait", arguments: { slot } };
	output.content.push(toolCall);
	stream.push({ type: "toolcall_start", contentIndex, partial: output });
	stream.push({ type: "toolcall_delta", contentIndex, delta: JSON.stringify({ slot }), partial: output });
	stream.push({ type: "toolcall_end", contentIndex, toolCall, partial: output });
}

function streamProbe(
	model: Model<Api>,
	context: Context,
	options?: SimpleStreamOptions,
): AssistantMessageEventStream {
	const stream = createAssistantMessageEventStream();
	void (async () => {
		const output = newMessage(model);
		try {
			if (options?.signal?.aborted) throw abortError();
			stream.push({ type: "start", partial: output });
			const fixtureUserTexts = context.messages
				.map(userText)
				.filter((text): text is string => text !== undefined)
				.map(projectFixtureText);
			const directory = probeDirectory();
			const countFile = join(directory, "model-call-count");
			const previousCount = existsSync(countFile)
				? Number.parseInt(await readFile(countFile, "utf8"), 10) || 0
				: 0;
			await mark("model-call-count", `${previousCount + 1}\n`);
			if (context.systemPrompt?.includes("CONTEXT_NONCE")) await mark("context-observed");
			if (fixtureUserTexts.some((text) => text.includes("STEERING_NONCE"))) await mark("steering-observed");
			if (fixtureUserTexts.some((text) => text.includes("TEMPLATE_NONCE"))) await mark("template-observed");
			if (fixtureUserTexts.some((text) => text.includes("SKILL_NONCE"))) await mark("skill-observed");

			if (fixtureUserTexts.length === 1 && fixtureUserTexts[0] === "PROBE_TOOL_BATCH") {
				pushToolCall(stream, output, "batch-a", "a");
				pushToolCall(stream, output, "batch-b", "b");
				output.stopReason = "toolUse";
			} else {
				const artifact = {
					provider: model.provider,
					model: model.id,
					userTexts: fixtureUserTexts,
					toolAFinished: existsSync(join(directory, "tool-a-finished")),
					toolBFinished: existsSync(join(directory, "tool-b-finished")),
					toolResultIds: context.messages
						.filter((message) => message.role === "toolResult")
						.map((message) => message.toolCallId),
				};
				await mark("probe.json", `${JSON.stringify(artifact, null, 2)}\n`);
				const marker = fixtureUserTexts.at(-1) ?? "NO_USER_TEXT";
				const text = `ACK:${marker}`;
				const contentIndex = output.content.length;
				output.content.push({ type: "text", text });
				stream.push({ type: "text_start", contentIndex, partial: output });
				stream.push({ type: "text_delta", contentIndex, delta: text, partial: output });
				stream.push({ type: "text_end", contentIndex, content: text, partial: output });
				output.stopReason = "stop";
			}

			stream.push({ type: "done", reason: output.stopReason, message: output });
			stream.end();
		} catch (error) {
			output.stopReason = options?.signal?.aborted ? "aborted" : "error";
			output.errorMessage = error instanceof Error ? error.message : String(error);
			stream.push({ type: "error", reason: output.stopReason, error: output });
			stream.end();
		}
	})();
	return stream;
}

export default function (pi: ExtensionAPI): void {
	pi.on("session_before_switch", async () => {
		if (!existsSync(join(probeDirectory(), "hold-switch"))) return;
		await mark("switch-entered");
		await waitForRelease("switch", new AbortController().signal);
	});

	pi.registerProvider(PROVIDER, {
		baseUrl: "http://127.0.0.1.invalid",
		apiKey: "fixture-no-network",
		api: "claudine-pi-probe" as Api,
		models: [
			{
				id: MODEL,
				name: "Claudine deterministic Pi steering probe",
				reasoning: false,
				input: ["text"],
				cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
				contextWindow: 8_192,
				maxTokens: 1_024,
			},
		],
		streamSimple: streamProbe,
	});

	pi.registerTool({
		name: "probe_wait",
		label: "Probe wait",
		description: "Wait for a deterministic steering-test release gate.",
		parameters: Type.Object({ slot: Type.Union([Type.Literal("a"), Type.Literal("b")]) }),
		async execute(_toolCallId, params, signal) {
			await mark(`tool-${params.slot}-started`);
			await waitForRelease(params.slot, signal);
			await mark(`tool-${params.slot}-finished`);
			return { content: [{ type: "text", text: `released-${params.slot}` }], details: { slot: params.slot } };
		},
	});

	pi.registerCommand("probe-handled", {
		description: "Handle a deterministic RPC probe without invoking the model.",
		handler: async (_args, ctx) => {
			await mark("extension-handled");
			ctx.ui.notify("probe-handled", "info");
		},
	});

	pi.registerCommand("probe-ui", {
		description: "Request an RPC confirmation for deterministic UI testing.",
		handler: async (_args, ctx) => {
			const confirmed = await ctx.ui.confirm("Pi probe", "Approve the fixture probe?");
			await mark("ui-confirm-result", `${confirmed}\n`);
		},
	});
}
