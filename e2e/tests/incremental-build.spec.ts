import { expect } from "@playwright/test";
import { createTestWithFixture } from "./test-utils";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Create test instance with incremental-build fixture
const test = createTestWithFixture("incremental-build");

// Allow retries for timing-sensitive tests
test.describe.configure({ mode: "serial", retries: 2 });

/**
 * Wait for dev server to complete a build by looking for specific patterns.
 * Waits for the build to START, then waits for it to FINISH.
 */
async function waitForBuildComplete(devServer: any, timeoutMs = 30000): Promise<string[]> {
	const startTime = Date.now();
	
	// Phase 1: Wait for build to start
	while (Date.now() - startTime < timeoutMs) {
		const logs = devServer.getLogs(200);
		const logsText = logs.join("\n").toLowerCase();
		
		if (logsText.includes("rerunning") || 
		    logsText.includes("rebuilding") ||
		    logsText.includes("files changed")) {
			break;
		}
		
		await new Promise(resolve => setTimeout(resolve, 50));
	}
	
	// Phase 2: Wait for build to finish
	while (Date.now() - startTime < timeoutMs) {
		const logs = devServer.getLogs(200);
		const logsText = logs.join("\n").toLowerCase();
		
		if (logsText.includes("finished") || 
		    logsText.includes("rerun finished") ||
		    logsText.includes("build finished")) {
			// Wait for filesystem to fully sync
			await new Promise(resolve => setTimeout(resolve, 500));
			return devServer.getLogs(200);
		}
		
		await new Promise(resolve => setTimeout(resolve, 100));
	}
	
	throw new Error(`Build did not complete within ${timeoutMs}ms`);
}

/**
 * Extract the build ID from an HTML file.
 */
function getBuildId(htmlPath: string): string | null {
	try {
		const content = readFileSync(htmlPath, "utf-8");
		const match = content.match(/data-build-id="(\d+)"/);
		return match ? match[1] : null;
	} catch {
		return null;
	}
}

/**
 * Check if logs indicate incremental build was used
 */
function isIncrementalBuild(logs: string[]): boolean {
	return logs.join("\n").toLowerCase().includes("incremental build");
}

/**
 * Get the number of affected routes from logs
 */
function getAffectedRouteCount(logs: string[]): number {
	const logsText = logs.join("\n");
	const match = logsText.match(/Rebuilding (\d+) affected routes/i);
	return match ? parseInt(match[1], 10) : -1;
}

test.describe("Incremental Build", () => {
	test.setTimeout(180000);

	const fixturePath = resolve(__dirname, "..", "fixtures", "incremental-build");
	
	// Asset paths
	const blogStylesPath = resolve(fixturePath, "src", "assets", "blog.css");
	
	// Output HTML paths
	const htmlPaths = {
		index: resolve(fixturePath, "dist", "index.html"),
		about: resolve(fixturePath, "dist", "about", "index.html"),
		blog: resolve(fixturePath, "dist", "blog", "index.html"),
	};
	
	// Original content storage
	let originalBlogStyles: string;

	test.beforeAll(async () => {
		originalBlogStyles = readFileSync(blogStylesPath, "utf-8");
		// Ensure file is in original state
		writeFileSync(blogStylesPath, originalBlogStyles, "utf-8");
	});

	test.afterAll(async () => {
		// Restore original content
		writeFileSync(blogStylesPath, originalBlogStyles, "utf-8");
	});

	test("incremental builds only rebuild affected routes", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			writeFileSync(blogStylesPath, originalBlogStyles + `\n/* test-${testCounter}-${suffix} */`, "utf-8");
			return await waitForBuildComplete(devServer, 30000);
		}
		
		// ========================================
		// SETUP: Establish incremental build state
		// ========================================
		// First change triggers a full build (no previous state)
		await triggerChange("init");
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Second change should be incremental (state now exists)
		let logs = await triggerChange("setup");
		expect(isIncrementalBuild(logs)).toBe(true);
		await new Promise(resolve => setTimeout(resolve, 500));

		// ========================================
		// TEST: CSS file change (blog.css → only /blog)
		// ========================================
		// Record build IDs before
		const beforeIndex = getBuildId(htmlPaths.index);
		const beforeAbout = getBuildId(htmlPaths.about);
		const beforeBlog = getBuildId(htmlPaths.blog);
		
		expect(beforeIndex).not.toBeNull();
		expect(beforeAbout).not.toBeNull();
		expect(beforeBlog).not.toBeNull();
		
		// Wait a bit more to ensure clean slate
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Trigger the change
		logs = await triggerChange("final");
		
		// Verify it was an incremental build
		expect(isIncrementalBuild(logs)).toBe(true);
		
		// Verify exactly 1 route was rebuilt (from logs)
		const routeCount = getAffectedRouteCount(logs);
		expect(routeCount).toBe(1);
		
		// Verify build IDs: only blog should have changed
		const afterIndex = getBuildId(htmlPaths.index);
		const afterAbout = getBuildId(htmlPaths.about);
		const afterBlog = getBuildId(htmlPaths.blog);
		
		// Index and about should NOT have been rebuilt
		expect(afterIndex).toBe(beforeIndex);
		expect(afterAbout).toBe(beforeAbout);
		
		// Blog SHOULD have been rebuilt
		expect(afterBlog).not.toBe(beforeBlog);
	});
});
