import { expect } from "@playwright/test";
import { createTestWithFixture } from "./test-utils";
import { readFileSync, writeFileSync, copyFileSync } from "node:fs";
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

/**
 * Helper to set up incremental build state
 */
async function setupIncrementalState(
	devServer: any,
	triggerChange: (suffix: string) => Promise<string[]>
): Promise<void> {
	// First change triggers a full build (no previous state)
	await triggerChange("init");
	await new Promise(resolve => setTimeout(resolve, 500));
	
	// Second change should be incremental (state now exists)
	const logs = await triggerChange("setup");
	expect(isIncrementalBuild(logs)).toBe(true);
	await new Promise(resolve => setTimeout(resolve, 500));
}

/**
 * Record build IDs for all pages
 */
function recordBuildIds(htmlPaths: Record<string, string>): Record<string, string | null> {
	const ids: Record<string, string | null> = {};
	for (const [name, path] of Object.entries(htmlPaths)) {
		ids[name] = getBuildId(path);
	}
	return ids;
}

test.describe("Incremental Build", () => {
	test.setTimeout(180000);

	const fixturePath = resolve(__dirname, "..", "fixtures", "incremental-build");
	
	// Asset paths
	const assets = {
		blogCss: resolve(fixturePath, "src", "assets", "blog.css"),
		utilsJs: resolve(fixturePath, "src", "assets", "utils.js"),
		mainJs: resolve(fixturePath, "src", "assets", "main.js"),
		aboutJs: resolve(fixturePath, "src", "assets", "about.js"),
		stylesCss: resolve(fixturePath, "src", "assets", "styles.css"),
		logoPng: resolve(fixturePath, "src", "assets", "logo.png"),
		teamPng: resolve(fixturePath, "src", "assets", "team.png"),
		bgPng: resolve(fixturePath, "src", "assets", "bg.png"),
	};
	
	// Output HTML paths
	const htmlPaths = {
		index: resolve(fixturePath, "dist", "index.html"),
		about: resolve(fixturePath, "dist", "about", "index.html"),
		blog: resolve(fixturePath, "dist", "blog", "index.html"),
	};
	
	// Original content storage
	const originals: Record<string, string | Buffer> = {};

	test.beforeAll(async () => {
		// Store original content for all assets we might modify
		originals.blogCss = readFileSync(assets.blogCss, "utf-8");
		originals.utilsJs = readFileSync(assets.utilsJs, "utf-8");
		originals.mainJs = readFileSync(assets.mainJs, "utf-8");
		originals.aboutJs = readFileSync(assets.aboutJs, "utf-8");
		originals.stylesCss = readFileSync(assets.stylesCss, "utf-8");
		originals.logoPng = readFileSync(assets.logoPng); // binary
		originals.teamPng = readFileSync(assets.teamPng); // binary
		originals.bgPng = readFileSync(assets.bgPng); // binary
	});

	test.afterAll(async () => {
		// Restore all original content
		writeFileSync(assets.blogCss, originals.blogCss);
		writeFileSync(assets.utilsJs, originals.utilsJs);
		writeFileSync(assets.mainJs, originals.mainJs);
		writeFileSync(assets.aboutJs, originals.aboutJs);
		writeFileSync(assets.stylesCss, originals.stylesCss);
		writeFileSync(assets.logoPng, originals.logoPng);
		writeFileSync(assets.teamPng, originals.teamPng);
		writeFileSync(assets.bgPng, originals.bgPng);
	});

	// ============================================================
	// TEST 1: Direct CSS dependency (blog.css → /blog only)
	// ============================================================
	test("CSS file change rebuilds only routes using it", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			writeFileSync(assets.blogCss, originals.blogCss + `\n/* test-${testCounter}-${suffix} */`);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		// Record build IDs before
		const before = recordBuildIds(htmlPaths);
		expect(before.index).not.toBeNull();
		expect(before.about).not.toBeNull();
		expect(before.blog).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Trigger the change
		const logs = await triggerChange("final");
		
		// Verify incremental build with 1 route
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(1);
		
		// Verify only blog was rebuilt
		const after = recordBuildIds(htmlPaths);
		expect(after.index).toBe(before.index);
		expect(after.about).toBe(before.about);
		expect(after.blog).not.toBe(before.blog);
	});

	// ============================================================
	// TEST 2: Transitive JS dependency (utils.js → main.js → /)
	// ============================================================
	test("transitive JS dependency change rebuilds affected routes", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			writeFileSync(assets.utilsJs, originals.utilsJs + `\n// test-${testCounter}-${suffix}`);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.index).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build with 1 route
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(1);
		
		// Only index should be rebuilt (uses main.js which imports utils.js)
		const after = recordBuildIds(htmlPaths);
		expect(after.about).toBe(before.about);
		expect(after.blog).toBe(before.blog);
		expect(after.index).not.toBe(before.index);
	});

	// ============================================================
	// TEST 3: Direct JS entry point change (about.js → /about)
	// ============================================================
	test("direct JS entry point change rebuilds only routes using it", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			writeFileSync(assets.aboutJs, originals.aboutJs + `\n// test-${testCounter}-${suffix}`);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.about).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build with 1 route
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(1);
		
		// Only about should be rebuilt
		const after = recordBuildIds(htmlPaths);
		expect(after.index).toBe(before.index);
		expect(after.blog).toBe(before.blog);
		expect(after.about).not.toBe(before.about);
	});

	// ============================================================
	// TEST 4: Shared asset change (styles.css → / AND /about)
	// ============================================================
	test("shared asset change rebuilds all routes using it", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			writeFileSync(assets.stylesCss, originals.stylesCss + `\n/* test-${testCounter}-${suffix} */`);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.index).not.toBeNull();
		expect(before.about).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build with 2 routes (/ and /about both use styles.css)
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(2);
		
		// Index and about should be rebuilt, blog should not
		const after = recordBuildIds(htmlPaths);
		expect(after.blog).toBe(before.blog);
		expect(after.index).not.toBe(before.index);
		expect(after.about).not.toBe(before.about);
	});

	// ============================================================
	// TEST 5: Image change (logo.png → /)
	// ============================================================
	test("image change rebuilds only routes using it", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			// For images, we append bytes to change the file
			// This simulates modifying an image file
			const modified = Buffer.concat([
				originals.logoPng as Buffer,
				Buffer.from(`<!-- test-${testCounter}-${suffix} -->`)
			]);
			writeFileSync(assets.logoPng, modified);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.index).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build with 1 route
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(1);
		
		// Only index should be rebuilt (uses logo.png)
		const after = recordBuildIds(htmlPaths);
		expect(after.about).toBe(before.about);
		expect(after.blog).toBe(before.blog);
		expect(after.index).not.toBe(before.index);
	});

	// ============================================================
	// TEST 6: Multiple files changed simultaneously
	// ============================================================
	test("multiple file changes rebuild union of affected routes", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			// Change both blog.css (affects /blog) and about.js (affects /about)
			writeFileSync(assets.blogCss, originals.blogCss + `\n/* test-${testCounter}-${suffix} */`);
			writeFileSync(assets.aboutJs, originals.aboutJs + `\n// test-${testCounter}-${suffix}`);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.about).not.toBeNull();
		expect(before.blog).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build with 2 routes (/about and /blog)
		expect(isIncrementalBuild(logs)).toBe(true);
		expect(getAffectedRouteCount(logs)).toBe(2);
		
		// About and blog should be rebuilt, index should not
		const after = recordBuildIds(htmlPaths);
		expect(after.index).toBe(before.index);
		expect(after.about).not.toBe(before.about);
		expect(after.blog).not.toBe(before.blog);
	});

	// ============================================================
	// TEST 7: CSS url() asset dependency (bg.png via blog.css → /blog)
	// ============================================================
	test("CSS url() asset change triggers rebundling and rebuilds affected routes", async ({ devServer }) => {
		let testCounter = 0;
		
		async function triggerChange(suffix: string) {
			testCounter++;
			devServer.clearLogs();
			// Modify bg.png - this is referenced via url() in blog.css
			// Changing it should trigger rebundling and rebuild /blog
			const modified = Buffer.concat([
				originals.bgPng as Buffer,
				Buffer.from(`<!-- test-${testCounter}-${suffix} -->`)
			]);
			writeFileSync(assets.bgPng, modified);
			return await waitForBuildComplete(devServer, 30000);
		}
		
		await setupIncrementalState(devServer, triggerChange);

		const before = recordBuildIds(htmlPaths);
		expect(before.blog).not.toBeNull();
		
		await new Promise(resolve => setTimeout(resolve, 500));
		
		const logs = await triggerChange("final");
		
		// Verify incremental build triggered
		expect(isIncrementalBuild(logs)).toBe(true);
		
		// Blog should be rebuilt (uses blog.css which references bg.png via url())
		// The bundler should have been re-run to update the hashed asset reference
		const after = recordBuildIds(htmlPaths);
		expect(after.blog).not.toBe(before.blog);
	});
});
