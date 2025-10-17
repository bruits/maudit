#!/usr/bin/env node

import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

// Map platform/arch to package names
function getBinaryPackageName(): string | null {
	const platform = process.platform;
	const arch = process.arch;

	// Map to package name format
	const packageMap: Record<string, string> = {
		"darwin-arm64": "@bruits/maudit-sidecar-darwin-arm64",
		"linux-x64": "@bruits/maudit-sidecar-linux-x64",
		"win32-x64": "@bruits/maudit-sidecar-windows-x64",
	};

	const key = `${platform}-${arch}`;
	return packageMap[key] || null;
}

// Get the binary path from the optional dependency
function getBinaryPath(): string | null {
	const packageName = getBinaryPackageName();
	if (!packageName) {
		console.error(`Unsupported platform: ${process.platform}-${process.arch}`);
		return null;
	}

	try {
		// Try to resolve the package directory by looking for package.json
		const packageJsonPath = require.resolve(`${packageName}/package.json`);
		const binaryDir = dirname(packageJsonPath);

		// Binary name depends on platform
		const binaryName =
			process.platform === "win32" ? "maudit-sidecar.exe" : "maudit-sidecar";
		const binaryPath = join(binaryDir, binaryName);

		return binaryPath;
	} catch (error) {
		console.error(
			`Failed to find binary for ${packageName}. Make sure the optional dependency is installed.`
		);
		console.error(error);
		return null;
	}
}

// Main execution
async function main() {
	const binaryPath = getBinaryPath();

	if (!binaryPath) {
		console.warn("No binary found, falling back to JS implementation");
		await import("./index.js");
		return;
	}

	// Spawn the binary and pass through stdio
	const child = spawn(binaryPath, process.argv.slice(2), {
		stdio: "inherit",
	});

	child.on("exit", (code) => {
		process.exit(code || 0);
	});

	child.on("error", async (error) => {
		console.error("Failed to start sidecar binary:", error);
		console.error("Falling back to JS implementation");
		await import("./index.js");
	});
}

main();
