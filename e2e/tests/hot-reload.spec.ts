import { expect } from "@playwright/test";
import { createTestWithFixture } from "./test-utils";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Create test instance with hot-reload fixture
const test = createTestWithFixture("hot-reload");

test.describe.configure({ mode: "serial" });

/**
 * Wait for dev server to complete a build/rerun by polling logs
 */
async function waitForBuildComplete(devServer: any, timeoutMs = 20000): Promise<string[]> {
	const startTime = Date.now();
	
	while (Date.now() - startTime < timeoutMs) {
		const logs = devServer.getLogs(100);
		const logsText = logs.join("\n").toLowerCase();
		
		// Look for completion messages
		if (logsText.includes("finished") || 
		    logsText.includes("rerun finished") ||
		    logsText.includes("build finished")) {
			return logs;
		}
		
		// Wait 100ms before checking again
		await new Promise(resolve => setTimeout(resolve, 100));
	}
	
	throw new Error(`Build did not complete within ${timeoutMs}ms`);
}

test.describe("Hot Reload", () => {
	// Increase timeout for these tests since they involve compilation
	test.setTimeout(60000);

	const fixturePath = resolve(__dirname, "..", "fixtures", "hot-reload");
	const indexPath = resolve(fixturePath, "src", "pages", "index.rs");
	const mainPath = resolve(fixturePath, "src", "main.rs");
	const dataPath = resolve(fixturePath, "data.txt");
	let originalIndexContent: string;
	let originalMainContent: string;
	let originalDataContent: string;

	test.beforeAll(async () => {
		// Save original content
		originalIndexContent = readFileSync(indexPath, "utf-8");
		originalMainContent = readFileSync(mainPath, "utf-8");
		originalDataContent = readFileSync(dataPath, "utf-8");

		// Ensure files are in original state
		writeFileSync(indexPath, originalIndexContent, "utf-8");
		writeFileSync(mainPath, originalMainContent, "utf-8");
		writeFileSync(dataPath, originalDataContent, "utf-8");
	});

	test.afterEach(async ({ devServer }) => {
		// Restore original content after each test
		writeFileSync(indexPath, originalIndexContent, "utf-8");
		writeFileSync(mainPath, originalMainContent, "utf-8");
		writeFileSync(dataPath, originalDataContent, "utf-8");
		
		// Only wait for build if devServer is available (startup might have failed)
		if (devServer) {
			try {
				devServer.clearLogs();
				await waitForBuildComplete(devServer);
			} catch (error) {
				console.warn("Failed to wait for build completion in afterEach:", error);
			}
		}
	});

	test.afterAll(async () => {
		// Restore original content
		writeFileSync(indexPath, originalIndexContent, "utf-8");
		writeFileSync(mainPath, originalMainContent, "utf-8");
		writeFileSync(dataPath, originalDataContent, "utf-8");
	});

	test("should recompile when Rust code changes (dependencies)", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		// Verify initial content
		await expect(page.locator("#title")).toHaveText("Original Title");

		// Clear logs to track what happens after this point
		devServer.clearLogs();

		// Modify main.rs - this is a tracked dependency, should trigger recompile
		const modifiedMain = originalMainContent.replace(
			"BuildOptions::default()",
			"BuildOptions::default() // Modified comment",
		);
		writeFileSync(mainPath, modifiedMain, "utf-8");

		// Wait for rebuild to complete
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n");

		// Check logs to verify it actually recompiled (ran cargo)
		expect(logsText).toContain("rebuilding");
		// Make sure it didn't just rerun the binary
		expect(logsText.toLowerCase()).not.toContain("rerunning binary");
	});

	test("should rerun without recompile when non-dependency files change", async ({
		page,
		devServer,
	}) => {
		await page.goto(devServer.url);

		// Verify initial content
		await expect(page.locator("#title")).toHaveText("Original Title");

		// Clear logs to track what happens after this point
		devServer.clearLogs();

		// Modify data.txt - this file is NOT in the .d dependencies
		// So it should trigger a rerun without recompilation
		writeFileSync(dataPath, "Modified data", "utf-8");

		// Wait for build/rerun to complete
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n");

		// Should see "rerunning binary" message (case insensitive)
		const hasRerunMessage = logsText.toLowerCase().includes("rerunning binary");
		expect(hasRerunMessage).toBe(true);

		// Should NOT see "rebuilding" message
		expect(logsText.toLowerCase()).not.toContain("rebuilding");
	});

	test("should show updated content after file changes", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		// Verify initial content
		await expect(page.locator("#title")).toHaveText("Original Title");

		// Prepare to wait for actual reload by waiting for the same URL to reload
		const currentUrl = page.url();

		// Modify the file
		const modifiedContent = originalIndexContent.replace(
			'h1 id="title" { "Original Title" }',
			'h1 id="title" { "Another Update" }',
		);
		writeFileSync(indexPath, modifiedContent, "utf-8");

		// Wait for the page to actually reload on the same URL
		await page.waitForURL(currentUrl, { timeout: 15000 });
		// Verify the updated content
		await expect(page.locator("#title")).toHaveText("Another Update", { timeout: 15000 });
	});
});
