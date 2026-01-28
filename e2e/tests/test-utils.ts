import { spawn } from "node:child_process";
import { resolve, dirname } from "node:path";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test as base } from "@playwright/test";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export interface DevServerOptions {
	/** Path to the fixture directory relative to e2e/fixtures/ */
	fixture: string;
	/** Port to run the server on (default: auto-find) */
	port?: number;
	/** Additional CLI flags to pass to maudit dev */
	flags?: string[];
}

export interface DevServer {
	/** Base URL of the dev server */
	url: string;
	/** Port the server is running on */
	port: number;
	/** Stop the dev server */
	stop: () => Promise<void>;
}

/**
 * Start a maudit dev server for testing.
 */
export async function startDevServer(options: DevServerOptions): Promise<DevServer> {
	// Use __dirname (test file location) to reliably find paths
	const e2eRoot = resolve(__dirname, "..");
	const fixturePath = resolve(e2eRoot, "fixtures", options.fixture);
	const flags = options.flags || [];
	const command = resolve(e2eRoot, "..", "target", "debug", "maudit");

	// Verify the binary exists
	if (!existsSync(command)) {
		throw new Error(
			`Maudit binary not found at: ${command}. Please build it with 'cargo build --bin maudit'`,
		);
	}

	// Build args array
	const args = ["dev", ...flags];
	if (options.port) {
		args.push("--port", options.port.toString());
	}

	// Start the dev server process
	const childProcess = spawn(command, args, {
		cwd: fixturePath,
		stdio: ["ignore", "pipe", "pipe"],
	});

	// Capture output to detect when server is ready
	let serverReady = false;

	const outputPromise = new Promise<number>((resolve, reject) => {
		const timeout = setTimeout(() => {
			reject(new Error("Dev server did not start within 30 seconds"));
		}, 30000);

		childProcess.stdout?.on("data", (data: Buffer) => {
			const output = data.toString();

			// Look for "waiting for requests" to know server is ready
			if (output.includes("waiting for requests")) {
				serverReady = true;
				clearTimeout(timeout);
				// We already know the port from options, so just resolve with it
				resolve(options.port || 1864);
			}
		});

		childProcess.stderr?.on("data", (data: Buffer) => {
			// Only log errors, not all stderr output
			const output = data.toString();
			if (output.toLowerCase().includes("error")) {
				console.error(`[maudit dev] ${output}`);
			}
		});

		childProcess.on("error", (error) => {
			clearTimeout(timeout);
			reject(new Error(`Failed to start dev server: ${error.message}`));
		});

		childProcess.on("exit", (code) => {
			if (!serverReady) {
				clearTimeout(timeout);
				reject(new Error(`Dev server exited with code ${code} before becoming ready`));
			}
		});
	});

	const port = await outputPromise;

	return {
		url: `http://127.0.0.1:${port}`,
		port,
		stop: async () => {
			return new Promise((resolve) => {
				childProcess.on("exit", () => resolve());
				childProcess.kill("SIGTERM");

				// Force kill after 5 seconds if it doesn't stop gracefully
				setTimeout(() => {
					if (!childProcess.killed) {
						childProcess.kill("SIGKILL");
					}
				}, 5000);
			});
		},
	};
}

/**
 * Helper to manage multiple dev servers in tests.
 * Automatically cleans up servers when tests finish.
 */
export class DevServerPool {
	private servers: DevServer[] = [];

	async start(options: DevServerOptions): Promise<DevServer> {
		const server = await startDevServer(options);
		this.servers.push(server);
		return server;
	}

	async stopAll(): Promise<void> {
		await Promise.all(this.servers.map((server) => server.stop()));
		this.servers = [];
	}
}

// Worker-scoped server pool - one server per worker, shared across all tests in that worker
// Key format: "workerIndex-fixtureName"
const workerServers = new Map<string, DevServer>();

/**
 * Create a test instance with a devServer fixture for a specific fixture.
 * This allows each test file to use a different fixture while sharing the same pattern.
 *
 * @param fixtureName - Name of the fixture directory under e2e/fixtures/
 * @param basePort - Starting port number (default: 1864). Each worker gets basePort + workerIndex
 *
 * @example
 * ```ts
 * import { createTestWithFixture } from "./test-utils";
 * const test = createTestWithFixture("my-fixture");
 *
 * test("my test", async ({ devServer }) => {
 *   // devServer is automatically started for "my-fixture"
 * });
 * ```
 */
export function createTestWithFixture(fixtureName: string, basePort = 1864) {
	return base.extend<{ devServer: DevServer }>({
		// oxlint-disable-next-line no-empty-pattern
		devServer: async ({}, use, testInfo) => {
			// Use worker index to get or create a server for this worker
			const workerIndex = testInfo.workerIndex;
			const serverKey = `${workerIndex}-${fixtureName}`;

			let server = workerServers.get(serverKey);

			if (!server) {
				// Assign unique port based on worker index
				const port = basePort + workerIndex;

				server = await startDevServer({
					fixture: fixtureName,
					port,
				});

				workerServers.set(serverKey, server);
			}

			await use(server);

			// Don't stop the server here - it stays alive for all tests in this worker
			// Playwright will clean up when the worker exits
		},
	});
}

export { expect } from "@playwright/test";
