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

test.describe("Hot Reload", () => {
	const fixturePath = resolve(__dirname, "..", "fixtures", "hot-reload");
	const indexPath = resolve(fixturePath, "src", "pages", "index.rs");
	const mainPath = resolve(fixturePath, "src", "main.rs");
	let originalIndexContent: string;
	let originalMainContent: string;

	test.beforeAll(async () => {
		// Save original content
		originalIndexContent = readFileSync(indexPath, "utf-8");
		originalMainContent = readFileSync(mainPath, "utf-8");
	});

	test.afterEach(async () => {
		// Restore original content after each test
		writeFileSync(indexPath, originalIndexContent, "utf-8");
		writeFileSync(mainPath, originalMainContent, "utf-8");
		// Wait a bit for the rebuild
		await new Promise((resolve) => setTimeout(resolve, 2000));
	});

	test.afterAll(async () => {
		// Restore original content
		writeFileSync(indexPath, originalIndexContent, "utf-8");
		writeFileSync(mainPath, originalMainContent, "utf-8");
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

		// Wait for rebuild to complete - look for "finished" in logs
		await new Promise((resolve) => {
			const checkInterval = setInterval(() => {
				const logs = devServer.getLogs(50).join("\n");
				if (logs.includes("finished") || logs.includes("Rebuild")) {
					clearInterval(checkInterval);
					resolve(null);
				}
			}, 100);

			// Timeout after 15 seconds
			setTimeout(() => {
				clearInterval(checkInterval);
				resolve(null);
			}, 15000);
		});

		// Check logs to verify it actually recompiled (ran cargo)
		const logs = devServer.getLogs(50).join("\n");
		expect(logs).toContain("rebuilding");
		expect(logs).not.toContain("Rerunning binary");
		expect(logs).not.toContain("rerunning binary");
	});

	test("should rerun without recompile when template changes (non-dependencies)", async ({ 
		page, 
		devServer 
	}) => {
		await page.goto(devServer.url);

		// Verify initial content
		await expect(page.locator("#title")).toHaveText("Original Title");

		// Prepare to wait for actual reload
		const currentUrl = page.url();

		// Clear logs to track what happens after this point
		devServer.clearLogs();

		// Modify the template in index.rs - this should NOT require recompilation
		// since it's just the HTML template, not the actual Rust code structure
		const modifiedContent = originalIndexContent.replace(
			'h1 id="title" { "Original Title" }',
			'h1 id="title" { "Template Updated" }',
		);
		writeFileSync(indexPath, modifiedContent, "utf-8");

		// Wait for the page to reload
		await page.waitForURL(currentUrl, { timeout: 15000 });
		
		// Verify the updated content
		await expect(page.locator("#title")).toHaveText("Template Updated", { timeout: 15000 });

		// Give logs time to be captured
		await new Promise((resolve) => setTimeout(resolve, 1000));

		// Check logs to verify it did NOT recompile
		const logs = devServer.getLogs(50).join("\n");
		
		// Should see "rerunning binary" or similar message
		const hasRerunMessage = logs.toLowerCase().includes("rerunning") || 
		                        logs.toLowerCase().includes("rerun");
		expect(hasRerunMessage).toBe(true);
		
		// Should NOT see cargo compilation messages
		expect(logs).not.toContain("Compiling");
		expect(logs.toLowerCase()).not.toContain("rebuilding");
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
