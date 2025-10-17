import * as readline from "node:readline";
import {
	handleTailwindMessage,
	type TailwindMessage,
	type TailwindResponse,
} from "./tailwind.js";

// Base message types that can be received from Rust parent
export interface BaseIncomingMessage {
	id?: string;
}

export interface BaseOutgoingMessage {
	type: "response" | "error";
	id?: string | undefined;
}

// Specific incoming message types
interface ReadyResponse extends BaseOutgoingMessage {
	ready: true;
}

interface ErrorResponse extends BaseOutgoingMessage {
	error: string;
}

type IncomingMessage = TailwindMessage;

type OutgoingMessage = TailwindResponse | ErrorResponse | ReadyResponse;

// Send a message back to the Rust parent
function sendMessage(message: OutgoingMessage): void {
	if ("error" in message && message.error) {
		// Send error: ERROR\n<length>\n<error message>\n
		const error = message.error;
		process.stdout.write(`ERROR\n${error.length}\n${error}\n`);
	} else if ("output" in message && message.output) {
		// Send success: OK\n<length>\n<css output>\n
		const output = message.output;
		process.stdout.write(`OK\n${output.length}\n${output}\n`);
	} else if ("ready" in message) {
		// Send ready: READY\n
		process.stdout.write("READY\n");
	}
}

// Handle incoming messages
async function handleMessage(message: IncomingMessage): Promise<void> {
	try {
		switch (message.type) {
			case "tailwind":
				const response = await handleTailwindMessage(message);
				sendMessage(response);
				return;
		}
	} catch (error) {
		const errorMsg = error instanceof Error ? error.message : String(error);
		const stackTrace = error instanceof Error ? error.stack : "";
		console.error("Error handling message:", errorMsg, stackTrace);
		sendMessage({
			type: "error",
			id: message.id,
			error: `${errorMsg}\n${stackTrace}`,
		});
	}
}

// Main message loop
async function main(): Promise<void> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
		terminal: false,
	});

	// Send ready signal
	sendMessage({ type: "response", ready: true });

	rl.on("line", async (line: string) => {
		try {
			const message = JSON.parse(line) as IncomingMessage;
			await handleMessage(message);
		} catch (error) {
			sendMessage({
				type: "error",
				error: `Failed to parse message: ${
					error instanceof Error ? error.message : String(error)
				}`,
			});
		}
	});

	rl.on("close", () => {
		process.exit(0);
	});
}

main().catch((error) => {
	console.error("Fatal error:", error);
	process.exit(1);
});
