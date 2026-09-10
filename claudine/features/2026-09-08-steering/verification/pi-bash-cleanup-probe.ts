import { mkdir, writeFile } from "node:fs/promises";
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

const PROVIDER = "claudine-bash-cleanup-probe";
const MODEL = "fixture";

function output(model: Model<Api>): AssistantMessage {
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

function streamProbe(
	model: Model<Api>,
	context: Context,
	_options?: SimpleStreamOptions,
): AssistantMessageEventStream {
	const stream = createAssistantMessageEventStream();
	void (async () => {
		const message = output(model);
		stream.push({ type: "start", partial: message });
		const hasToolResult = context.messages.some((entry) => entry.role === "toolResult");
		if (!hasToolResult) {
			const directory = process.env.CLAUDINE_PI_CLEANUP_DIR;
			const marker = process.env.CLAUDINE_PI_CLEANUP_MARKER;
			if (!directory || !marker) throw new Error("cleanup probe environment is required");
			await mkdir(directory, { recursive: true });
			await writeFile(
				join(directory, "resources.json"),
				`${JSON.stringify({
					context: context.systemPrompt?.includes("CLEANUP_CONTEXT_NONCE") ?? false,
					skill: context.systemPrompt?.includes("cleanup-probe-skill") ?? false,
				})}\n`,
				"utf8",
			);
			const command = 'echo $$ > "$CLAUDINE_PI_CLEANUP_DIR/shell.pid"; exec -a "$CLAUDINE_PI_CLEANUP_MARKER" sleep 20';
			const toolCall = { type: "toolCall" as const, id: "cleanup-bash", name: "bash", arguments: { command } };
			message.content.push(toolCall);
			stream.push({ type: "toolcall_start", contentIndex: 0, partial: message });
			stream.push({ type: "toolcall_delta", contentIndex: 0, delta: JSON.stringify({ command }), partial: message });
			stream.push({ type: "toolcall_end", contentIndex: 0, toolCall, partial: message });
			message.stopReason = "toolUse";
		} else {
			const text = "cleanup tool returned";
			message.content.push({ type: "text", text });
			stream.push({ type: "text_start", contentIndex: 0, partial: message });
			stream.push({ type: "text_delta", contentIndex: 0, delta: text, partial: message });
			stream.push({ type: "text_end", contentIndex: 0, content: text, partial: message });
			message.stopReason = "stop";
		}
		stream.push({ type: "done", reason: message.stopReason, message });
		stream.end();
	})();
	return stream;
}

export default function (pi: ExtensionAPI): void {
	pi.registerProvider(PROVIDER, {
		baseUrl: "http://127.0.0.1.invalid",
		apiKey: "fixture-no-network",
		api: "claudine-bash-cleanup" as Api,
		models: [{
			id: MODEL,
			name: "Claudine deterministic bash cleanup probe",
			reasoning: false,
			input: ["text"],
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
			contextWindow: 8_192,
			maxTokens: 1_024,
		}],
		streamSimple: streamProbe,
	});
}
