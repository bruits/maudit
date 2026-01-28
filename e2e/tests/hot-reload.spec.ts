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
	let originalContent: string;

	test.beforeAll(async () => {
		// Save original content
		originalContent = readFileSync(indexPath, "utf-8");
	});

	test.afterEach(async () => {
		// Restore original content after each test
		writeFileSync(indexPath, originalContent, "utf-8");
		// Wait a bit for the rebuild
		await new Promise((resolve) => setTimeout(resolve, 2000));
	});

	test.afterAll(async () => {
		// Restore original content
		writeFileSync(indexPath, originalContent, "utf-8");
	});

	test("should show updated content after file changes", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		// Verify initial content
		await expect(page.locator("#title")).toHaveText("Original Title");

		// Prepare to wait for actual reload by waiting for the same URL to reload
		const currentUrl = page.url();

		// Modify the file
		const modifiedContent = originalContent.replace(
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
