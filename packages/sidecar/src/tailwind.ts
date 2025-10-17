import { compile, optimize, type OptimizeOptions } from "@tailwindcss/node";
import { Scanner } from "@tailwindcss/oxide";
import { readFile } from "node:fs/promises";
import path from "node:path";
import type { BaseIncomingMessage, BaseOutgoingMessage } from "./index.js";

export interface TailwindMessage extends BaseIncomingMessage {
	type: "tailwind";
	inputFile: string;
	minify?: boolean;
}

export interface TailwindResponse extends BaseOutgoingMessage {
	output: string;
}

export async function handleTailwindMessage(
	message: TailwindMessage
): Promise<TailwindResponse> {
	let base = path.resolve(process.cwd());
	let inputFilePath = path.resolve(message.inputFile);
	let inputBasePath = path.dirname(inputFilePath);

	let fullRebuildPaths: string[] = inputFilePath ? [inputFilePath] : [];

	const input = await readFile(inputFilePath, "utf8");

	async function createCompiler(css: string) {
		let compiler = await compile(css, {
			base: inputBasePath,
			onDependency(path) {
				fullRebuildPaths.push(path);
			},
		});

		let sources = (() => {
			// Disable auto source detection
			if (compiler.root === "none") {
				return [];
			}

			// No root specified, use the base directory
			if (compiler.root === null) {
				return [{ base, pattern: "**/*", negated: false }];
			}

			// Use the specified root
			return [{ ...compiler.root, negated: false }];
		})().concat(compiler.sources);

		let scanner = new Scanner({ sources });
		return [compiler, scanner] as const;
	}

	let [compiler, scanner] = await handleError(() => createCompiler(input));

	let candidates = scanner.scan();

	let output = await handleError(() => compiler.build(candidates));

	if (message.minify) {
		const options: OptimizeOptions = {
			file: inputFilePath,
			minify: true,
		};

		let optimized = optimize(output, options);
		output = optimized.code;
	}

	return {
		type: "response",
		id: message.id,
		output: output,
	};
}

async function handleError<T>(fn: () => T): Promise<T> {
	try {
		return await fn();
	} catch (err) {
		if (err instanceof Error) {
			console.error(`Error in handleError: ${err.message}`);
			throw err;
		}
		throw new Error(String(err));
	}
}
