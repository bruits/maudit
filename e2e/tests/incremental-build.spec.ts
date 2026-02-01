import { expect } from "@playwright/test";
import { createTestWithFixture } from "./test-utils";
import { readFileSync, writeFileSync, renameSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Create test instance with incremental-build fixture
const test = createTestWithFixture("incremental-build");

// Run tests serially since they share state; allow retries for timing-sensitive tests
test.describe.configure({ mode: "serial", retries: 2 });

/**
 * Wait for dev server to complete a build by polling logs.
 * Returns logs once build is finished.
 */
async function waitForBuildComplete(devServer: any, timeoutMs = 30000): Promise<string[]> {
	const startTime = Date.now();
	const pollInterval = 50;

	// Phase 1: Wait for build to start
	while (Date.now() - startTime < timeoutMs) {
		const logs = devServer.getLogs(200);
		const logsText = logs.join("\n").toLowerCase();

		if (
			logsText.includes("rerunning") ||
			logsText.includes("rebuilding") ||
			logsText.includes("files changed")
		) {
			break;
		}

		await new Promise((r) => setTimeout(r, pollInterval));
	}

	// Phase 2: Wait for build to finish
	while (Date.now() - startTime < timeoutMs) {
		const logs = devServer.getLogs(200);
		const logsText = logs.join("\n").toLowerCase();

		if (
			logsText.includes("finished") ||
			logsText.includes("rerun finished") ||
			logsText.includes("build finished")
		) {
			return logs;
		}

		await new Promise((r) => setTimeout(r, pollInterval));
	}

	console.log("TIMEOUT - logs seen:", devServer.getLogs(50));
	throw new Error(`Build did not complete within ${timeoutMs}ms`);
}

/**
 * Wait for the dev server to become idle (no builds in progress).
 * This polls build IDs until they stop changing.
 */
async function waitForIdle(htmlPaths: Record<string, string>, stableMs = 200): Promise<void> {
	let lastIds = recordBuildIds(htmlPaths);
	let stableTime = 0;
	
	while (stableTime < stableMs) {
		await new Promise((r) => setTimeout(r, 50));
		const currentIds = recordBuildIds(htmlPaths);
		
		const allSame = Object.keys(lastIds).every(
			(key) => lastIds[key] === currentIds[key]
		);
		
		if (allSame) {
			stableTime += 50;
		} else {
			stableTime = 0;
			lastIds = currentIds;
		}
	}
}

/**
 * Wait for a specific HTML file's build ID to change from a known value.
 * This is more reliable than arbitrary sleeps.
 */
async function waitForBuildIdChange(
	htmlPath: string,
	previousId: string | null,
	timeoutMs = 30000,
): Promise<string> {
	const startTime = Date.now();
	const pollInterval = 50;

	while (Date.now() - startTime < timeoutMs) {
		const currentId = getBuildId(htmlPath);
		if (currentId !== null && currentId !== previousId) {
			// Small delay to let any concurrent writes settle
			await new Promise((r) => setTimeout(r, 100));
			return currentId;
		}
		await new Promise((r) => setTimeout(r, pollInterval));
	}

	throw new Error(`Build ID did not change within ${timeoutMs}ms`);
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
 * Record build IDs for all pages
 */
function recordBuildIds(htmlPaths: Record<string, string>): Record<string, string | null> {
	const ids: Record<string, string | null> = {};
	for (const [name, path] of Object.entries(htmlPaths)) {
		ids[name] = getBuildId(path);
	}
	return ids;
}

/**
 * Trigger a change and wait for build to complete.
 * Returns logs from the build.
 */
async function triggerAndWaitForBuild(
	devServer: any,
	modifyFn: () => void,
	timeoutMs = 30000,
): Promise<string[]> {
	devServer.clearLogs();
	modifyFn();
	return await waitForBuildComplete(devServer, timeoutMs);
}

/**
 * Set up incremental build state by triggering two builds.
 * First build establishes state, second ensures state is populated.
 * Returns build IDs recorded after the second build completes and server is idle.
 * 
 * Note: We don't assert incremental here - the actual test will verify that.
 * This is because on first test run the server might still be initializing.
 */
async function setupIncrementalState(
	devServer: any,
	modifyFn: (suffix: string) => void,
	htmlPaths: Record<string, string>,
	expectedChangedRoute: string, // Which route we expect to change
): Promise<Record<string, string | null>> {
	// First change: triggers build (establishes state)
	const beforeInit = getBuildId(htmlPaths[expectedChangedRoute]);
	await triggerAndWaitForBuild(devServer, () => modifyFn("init"));
	await waitForBuildIdChange(htmlPaths[expectedChangedRoute], beforeInit);

	// Second change: state should now exist for incremental builds
	const beforeSetup = getBuildId(htmlPaths[expectedChangedRoute]);
	await triggerAndWaitForBuild(devServer, () => modifyFn("setup"));
	await waitForBuildIdChange(htmlPaths[expectedChangedRoute], beforeSetup);
	
	// Wait for server to become completely idle before recording baseline
	await waitForIdle(htmlPaths);
	
	return recordBuildIds(htmlPaths);
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

		function modifyFile(suffix: string) {
			testCounter++;
			writeFileSync(assets.blogCss, originals.blogCss + `\n/* test-${testCounter}-${suffix} */`);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "blog");
		expect(before.index).not.toBeNull();
		expect(before.about).not.toBeNull();
		expect(before.blog).not.toBeNull();

		// Trigger the final change and wait for build
		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.blog, before.blog);

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

		function modifyFile(suffix: string) {
			testCounter++;
			writeFileSync(assets.utilsJs, originals.utilsJs + `\n// test-${testCounter}-${suffix}`);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "index");
		expect(before.index).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.index, before.index);

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

		function modifyFile(suffix: string) {
			testCounter++;
			writeFileSync(assets.aboutJs, originals.aboutJs + `\n// test-${testCounter}-${suffix}`);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "about");
		expect(before.about).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.about, before.about);

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

		function modifyFile(suffix: string) {
			testCounter++;
			writeFileSync(assets.stylesCss, originals.stylesCss + `\n/* test-${testCounter}-${suffix} */`);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "index");
		expect(before.index).not.toBeNull();
		expect(before.about).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.index, before.index);

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

		function modifyFile(suffix: string) {
			testCounter++;
			const modified = Buffer.concat([
				originals.logoPng as Buffer,
				Buffer.from(`<!-- test-${testCounter}-${suffix} -->`),
			]);
			writeFileSync(assets.logoPng, modified);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "index");
		expect(before.index).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.index, before.index);

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

		function modifyFile(suffix: string) {
			testCounter++;
			// Change both blog.css (affects /blog) and about.js (affects /about)
			writeFileSync(assets.blogCss, originals.blogCss + `\n/* test-${testCounter}-${suffix} */`);
			writeFileSync(assets.aboutJs, originals.aboutJs + `\n// test-${testCounter}-${suffix}`);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "blog");
		expect(before.about).not.toBeNull();
		expect(before.blog).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.blog, before.blog);

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
	test("CSS url() asset change triggers rebundling and rebuilds affected routes", async ({
		devServer,
	}) => {
		let testCounter = 0;

		function modifyFile(suffix: string) {
			testCounter++;
			const modified = Buffer.concat([
				originals.bgPng as Buffer,
				Buffer.from(`<!-- test-${testCounter}-${suffix} -->`),
			]);
			writeFileSync(assets.bgPng, modified);
		}

		const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "blog");
		expect(before.blog).not.toBeNull();

		const logs = await triggerAndWaitForBuild(devServer, () => modifyFile("final"));
		await waitForBuildIdChange(htmlPaths.blog, before.blog);

		// Verify incremental build triggered
		expect(isIncrementalBuild(logs)).toBe(true);

		// Blog should be rebuilt (uses blog.css which references bg.png via url())
		const after = recordBuildIds(htmlPaths);
		expect(after.blog).not.toBe(before.blog);
	});

	// ============================================================
	// TEST 8: Source file change rebuilds only routes defined in that file
	// ============================================================
	test("source file change rebuilds only routes defined in that file", async ({ devServer }) => {
		// This test verifies that when a .rs source file changes, only routes
		// defined in that file are rebuilt (via source_to_routes tracking).
		//
		// Flow:
		// 1. Dev server starts → initial build → creates build_state.json with source file mappings
		// 2. Modify about.rs → cargo recompiles → binary reruns with MAUDIT_CHANGED_FILES
		// 3. New binary loads build_state.json and finds /about is affected by about.rs
		// 4. Only /about route is rebuilt
		//
		// Note: Unlike asset changes, .rs changes require cargo recompilation.
		// The binary's logs (showing "Incremental build") aren't captured by the
		// dev server's log collection, so we verify behavior through build IDs.

		const aboutRs = resolve(fixturePath, "src", "pages", "about.rs");
		const originalAboutRs = readFileSync(aboutRs, "utf-8");

		try {
			let testCounter = 0;

			function modifyFile(suffix: string) {
				testCounter++;
				writeFileSync(aboutRs, originalAboutRs + `\n// test-${testCounter}-${suffix}`);
			}

			const rsTimeout = 60000;

			// First change: triggers recompile + build (establishes build state with source_to_routes)
			const beforeInit = getBuildId(htmlPaths.about);
			await triggerAndWaitForBuild(devServer, () => modifyFile("init"), rsTimeout);
			await waitForBuildIdChange(htmlPaths.about, beforeInit, rsTimeout);

			// Record build IDs - state now exists with source_to_routes mappings
			const before = recordBuildIds(htmlPaths);
			expect(before.index).not.toBeNull();
			expect(before.about).not.toBeNull();
			expect(before.blog).not.toBeNull();

			// Second change: should do incremental build (only about.rs route)
			await triggerAndWaitForBuild(devServer, () => modifyFile("final"), rsTimeout);
			await waitForBuildIdChange(htmlPaths.about, before.about, rsTimeout);

			// Verify only /about was rebuilt (it's defined in about.rs)
			const after = recordBuildIds(htmlPaths);
			expect(after.index).toBe(before.index);
			expect(after.blog).toBe(before.blog);
			expect(after.about).not.toBe(before.about);

		} finally {
			// Restore original content and wait for build to complete
			const beforeRestore = getBuildId(htmlPaths.about);
			writeFileSync(aboutRs, originalAboutRs);
			try {
				await waitForBuildIdChange(htmlPaths.about, beforeRestore, 60000);
			} catch {
				// Restoration build may not always complete, that's ok
			}
		}
	});

	// ============================================================
	// TEST 9: include_str! file change triggers full rebuild (untracked file)
	// ============================================================
	test("include_str file change triggers full rebuild", async ({ devServer }) => {
		// This test verifies that changing a file referenced by include_str!()
		// triggers cargo recompilation and a FULL rebuild (all routes).
		//
		// Setup: about.rs uses include_str!("../assets/about-content.txt")
		// The .d file from cargo includes this dependency, so the dependency tracker
		// knows that changing about-content.txt requires recompilation.
		//
		// Flow:
		// 1. Dev server starts → initial build
		// 2. Modify about-content.txt → cargo recompiles (because .d file tracks it)
		// 3. Binary runs with MAUDIT_CHANGED_FILES pointing to about-content.txt
		// 4. Since about-content.txt is NOT in source_to_routes or asset_to_routes,
		//    it's an "untracked file" and triggers a full rebuild of all routes
		//
		// This is the correct safe behavior - we don't know which route uses the
		// include_str! file, so we rebuild everything to ensure correctness.

		const contentFile = resolve(fixturePath, "src", "assets", "about-content.txt");
		const originalContent = readFileSync(contentFile, "utf-8");
		const rsTimeout = 60000;

		try {
			let testCounter = 0;

			function modifyFile(suffix: string) {
				testCounter++;
				writeFileSync(contentFile, originalContent + `\n<!-- test-${testCounter}-${suffix} -->`);
			}

			// First change: triggers recompile + full build (establishes build state)
			const beforeInit = getBuildId(htmlPaths.about);
			await triggerAndWaitForBuild(devServer, () => modifyFile("init"), rsTimeout);
			await waitForBuildIdChange(htmlPaths.about, beforeInit, rsTimeout);

			// Record build IDs before the final change
			const before = recordBuildIds(htmlPaths);
			expect(before.index).not.toBeNull();
			expect(before.about).not.toBeNull();
			expect(before.blog).not.toBeNull();

			// Trigger the content file change with unique content to verify
			devServer.clearLogs();
			writeFileSync(contentFile, originalContent + "\nUpdated content!");
			await waitForBuildComplete(devServer, rsTimeout);
			await waitForBuildIdChange(htmlPaths.about, before.about, rsTimeout);

			// All routes should be rebuilt (full rebuild due to untracked file)
			const after = recordBuildIds(htmlPaths);
			expect(after.index).not.toBe(before.index);
			expect(after.about).not.toBe(before.about);
			expect(after.blog).not.toBe(before.blog);

			// Verify the content was actually updated in the output
			const aboutHtml = readFileSync(htmlPaths.about, "utf-8");
			expect(aboutHtml).toContain("Updated content!");

		} finally {
			// Restore original content and wait for build to complete
			const beforeRestore = getBuildId(htmlPaths.about);
			writeFileSync(contentFile, originalContent);
			try {
				await waitForBuildIdChange(htmlPaths.about, beforeRestore, 60000);
			} catch {
				// Restoration build may not always complete, that's ok
			}
		}
	});

	// ============================================================
	// TEST 10: Folder rename detection
	// ============================================================
	test("folder rename is detected and affects routes using assets in that folder", async ({ devServer }) => {
		// This test verifies that renaming a folder containing tracked assets
		// is detected by the file watcher and affects the correct routes.
		//
		// Setup: The blog page uses src/assets/icons/blog-icon.css
		// Test: Rename icons -> icons-renamed, verify the blog route is identified as affected
		//
		// Note: The actual build will fail because the asset path becomes invalid,
		// but this test verifies the DETECTION and ROUTE MATCHING works correctly.

		const iconsFolder = resolve(fixturePath, "src", "assets", "icons");
		const renamedFolder = resolve(fixturePath, "src", "assets", "icons-renamed");
		const iconFile = resolve(iconsFolder, "blog-icon.css");

		// Ensure we start with the correct state
		if (existsSync(renamedFolder)) {
			renameSync(renamedFolder, iconsFolder);
			// Wait briefly for any triggered build to start
			await new Promise((resolve) => setTimeout(resolve, 500));
		}

		expect(existsSync(iconsFolder)).toBe(true);
		expect(existsSync(iconFile)).toBe(true);

		const originalContent = readFileSync(iconFile, "utf-8");

		try {
			let testCounter = 0;

			function modifyFile(suffix: string) {
				testCounter++;
				writeFileSync(iconFile, originalContent + `\n/* test-${testCounter}-${suffix} */`);
			}

			// Use setupIncrementalState to establish tracking
			const before = await setupIncrementalState(devServer, modifyFile, htmlPaths, "blog");
			expect(before.blog).not.toBeNull();

			// Clear logs for the actual test
			devServer.clearLogs();

			// Rename icons -> icons-renamed
			renameSync(iconsFolder, renamedFolder);

			// Wait for the build to be attempted (it will fail because path is now invalid)
			const startTime = Date.now();
			const timeoutMs = 15000;
			let logs: string[] = [];

			while (Date.now() - startTime < timeoutMs) {
				logs = devServer.getLogs(100);
				const logsText = logs.join("\n");

				// Wait for either success or failure indication
				if (logsText.includes("finished") || logsText.includes("failed") || logsText.includes("error")) {
					break;
				}

				await new Promise((resolve) => setTimeout(resolve, 100));
			}

			logs = devServer.getLogs(100);
			const logsText = logs.join("\n");

			// Key assertions: verify the detection and route matching worked
			// 1. The folder paths should be in changed files
			expect(logsText).toContain("icons");

			// 2. The blog route should be identified as affected
			expect(logsText).toContain("Rebuilding 1 affected routes");
			expect(logsText).toContain("/blog");

			// 3. Other routes should NOT be affected (index and about don't use icons/)
			expect(logsText).not.toContain("/about");

		} finally {
			// Restore: rename icons-renamed back to icons
			if (existsSync(renamedFolder) && !existsSync(iconsFolder)) {
				renameSync(renamedFolder, iconsFolder);
			}
			// Restore original content and wait for build
			if (existsSync(iconFile)) {
				const beforeRestore = getBuildId(htmlPaths.blog);
				writeFileSync(iconFile, originalContent);
				try {
					await waitForBuildIdChange(htmlPaths.blog, beforeRestore, 30000);
				} catch {
					// Restoration build may not always complete, that's ok
				}
			}
		}
	});

	// ============================================================
	// TEST 11: Shared Rust module change triggers full rebuild
	// ============================================================
	test("shared Rust module change triggers full rebuild", async ({ devServer }) => {
		// This test verifies that changing a shared Rust module (not a route file)
		// triggers a full rebuild of all routes.
		//
		// Setup: helpers.rs contains shared functions used by about.rs
		// The helpers.rs file is not tracked in source_to_routes (only route files are)
		// so it's treated as an "untracked file" which triggers a full rebuild.
		//
		// This is the correct safe behavior - we can't determine which routes
		// depend on the shared module, so we rebuild everything.

		const helpersRs = resolve(fixturePath, "src", "pages", "helpers.rs");
		const originalContent = readFileSync(helpersRs, "utf-8");
		const rsTimeout = 60000;

		try {
			let testCounter = 0;

			function modifyFile(suffix: string) {
				testCounter++;
				writeFileSync(helpersRs, originalContent + `\n// test-${testCounter}-${suffix}`);
			}

			// First change: triggers recompile + full build (establishes build state)
			const beforeInit = getBuildId(htmlPaths.index);
			await triggerAndWaitForBuild(devServer, () => modifyFile("init"), rsTimeout);
			await waitForBuildIdChange(htmlPaths.index, beforeInit, rsTimeout);

			// Record build IDs before the final change
			const before = recordBuildIds(htmlPaths);
			expect(before.index).not.toBeNull();
			expect(before.about).not.toBeNull();
			expect(before.blog).not.toBeNull();

			// Trigger the shared module change
			await triggerAndWaitForBuild(devServer, () => modifyFile("final"), rsTimeout);
			await waitForBuildIdChange(htmlPaths.index, before.index, rsTimeout);

			// All routes should be rebuilt (full rebuild due to untracked shared module)
			const after = recordBuildIds(htmlPaths);
			expect(after.index).not.toBe(before.index);
			expect(after.about).not.toBe(before.about);
			expect(after.blog).not.toBe(before.blog);

		} finally {
			// Restore original content and wait for build to complete
			const beforeRestore = getBuildId(htmlPaths.index);
			writeFileSync(helpersRs, originalContent);
			try {
				await waitForBuildIdChange(htmlPaths.index, beforeRestore, 60000);
			} catch {
				// Restoration build may not always complete, that's ok
			}
		}
	});
});
