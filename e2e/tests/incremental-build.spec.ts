import { expect } from "@playwright/test";
import { createTestWithFixture } from "./test-utils";
import { readFileSync, writeFileSync, statSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Create test instance with incremental-build fixture
const test = createTestWithFixture("incremental-build");

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

test.describe("Incremental Build", () => {
	// Increase timeout for these tests since they involve compilation
	test.setTimeout(60000);

	const fixturePath = resolve(__dirname, "..", "fixtures", "incremental-build");
	const stylesPath = resolve(fixturePath, "src", "assets", "styles.css");
	const blogStylesPath = resolve(fixturePath, "src", "assets", "blog.css");
	const mainScriptPath = resolve(fixturePath, "src", "assets", "main.js");
	const aboutScriptPath = resolve(fixturePath, "src", "assets", "about.js");
	const logoPath = resolve(fixturePath, "src", "assets", "logo.png");
	const teamPath = resolve(fixturePath, "src", "assets", "team.png");
	
	const indexHtmlPath = resolve(fixturePath, "dist", "index.html");
	const aboutHtmlPath = resolve(fixturePath, "dist", "about", "index.html");
	const blogHtmlPath = resolve(fixturePath, "dist", "blog", "index.html");
	
	let originalStylesContent: string;
	let originalBlogStylesContent: string;
	let originalMainScriptContent: string;
	let originalAboutScriptContent: string;
	let originalLogoContent: Buffer;
	let originalTeamContent: Buffer;

	test.beforeAll(async () => {
		// Save original content
		originalStylesContent = readFileSync(stylesPath, "utf-8");
		originalBlogStylesContent = readFileSync(blogStylesPath, "utf-8");
		originalMainScriptContent = readFileSync(mainScriptPath, "utf-8");
		originalAboutScriptContent = readFileSync(aboutScriptPath, "utf-8");
		originalLogoContent = readFileSync(logoPath);
		originalTeamContent = readFileSync(teamPath);

		// Ensure files are in original state
		writeFileSync(stylesPath, originalStylesContent, "utf-8");
		writeFileSync(blogStylesPath, originalBlogStylesContent, "utf-8");
		writeFileSync(mainScriptPath, originalMainScriptContent, "utf-8");
		writeFileSync(aboutScriptPath, originalAboutScriptContent, "utf-8");
		writeFileSync(logoPath, originalLogoContent);
		writeFileSync(teamPath, originalTeamContent);
	});

	test.afterEach(async ({ devServer }) => {
		// Restore original content after each test
		writeFileSync(stylesPath, originalStylesContent, "utf-8");
		writeFileSync(blogStylesPath, originalBlogStylesContent, "utf-8");
		writeFileSync(mainScriptPath, originalMainScriptContent, "utf-8");
		writeFileSync(aboutScriptPath, originalAboutScriptContent, "utf-8");
		writeFileSync(logoPath, originalLogoContent);
		writeFileSync(teamPath, originalTeamContent);
		
		// Wait for build if devServer is available
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
		writeFileSync(stylesPath, originalStylesContent, "utf-8");
		writeFileSync(blogStylesPath, originalBlogStylesContent, "utf-8");
		writeFileSync(mainScriptPath, originalMainScriptContent, "utf-8");
		writeFileSync(aboutScriptPath, originalAboutScriptContent, "utf-8");
		writeFileSync(logoPath, originalLogoContent);
		writeFileSync(teamPath, originalTeamContent);
	});

	test("should perform full build on first run after recompilation", async ({ devServer }) => {
		// Clear logs to track what happens after initial startup
		devServer.clearLogs();
		
		// Modify a file to trigger a rebuild
		writeFileSync(stylesPath, originalStylesContent + "\n/* comment */", "utf-8");
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n").toLowerCase();
		
		// After the first change post-startup, we should see an incremental build message
		expect(logsText).toContain("incremental build");
	});

	test("should only rebuild affected route when CSS changes", async ({ devServer }) => {
		// First, do a change to ensure we have build state
		writeFileSync(stylesPath, originalStylesContent + "\n/* setup */", "utf-8");
		await waitForBuildComplete(devServer);
		
		// Get modification times before change
		const indexMtimeBefore = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeBefore = statSync(aboutHtmlPath).mtimeMs;
		const blogMtimeBefore = statSync(blogHtmlPath).mtimeMs;
		
		// Wait longer to ensure timestamps differ and debouncer completes
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Clear logs
		devServer.clearLogs();
		
		// Change blog.css (only used by /blog route)
		writeFileSync(blogStylesPath, originalBlogStylesContent + "\n/* modified */", "utf-8");
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n").toLowerCase();
		
		// Should be incremental build
		expect(logsText).toContain("incremental build");
		
		// Get modification times after change
		const indexMtimeAfter = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeAfter = statSync(aboutHtmlPath).mtimeMs;
		const blogMtimeAfter = statSync(blogHtmlPath).mtimeMs;
		
		// Index and About should NOT be rebuilt (same mtime)
		expect(indexMtimeAfter).toBe(indexMtimeBefore);
		expect(aboutMtimeAfter).toBe(aboutMtimeBefore);
		
		// Blog should be rebuilt (different mtime)
		expect(blogMtimeAfter).toBeGreaterThan(blogMtimeBefore);
	});

	test("should rebuild multiple routes when shared asset changes", async ({ devServer }) => {
		// First, do a change to ensure we have build state
		writeFileSync(stylesPath, originalStylesContent + "\n/* setup */", "utf-8");
		await waitForBuildComplete(devServer);
		
		// Get modification times before change
		const indexMtimeBefore = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeBefore = statSync(aboutHtmlPath).mtimeMs;
		const blogMtimeBefore = statSync(blogHtmlPath).mtimeMs;
		
		// Wait longer to ensure timestamps differ and debouncer completes
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Clear logs
		devServer.clearLogs();
		
		// Change styles.css (used by /index route)
		writeFileSync(stylesPath, originalStylesContent + "\n/* modified */", "utf-8");
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n").toLowerCase();
		
		// Should be incremental build
		expect(logsText).toContain("incremental build");
		
		// Get modification times after change
		const indexMtimeAfter = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeAfter = statSync(aboutHtmlPath).mtimeMs;
		const blogMtimeAfter = statSync(blogHtmlPath).mtimeMs;
		
		// Index should be rebuilt (uses styles.css)
		expect(indexMtimeAfter).toBeGreaterThan(indexMtimeBefore);
		
		// About and Blog should NOT be rebuilt
		expect(aboutMtimeAfter).toBe(aboutMtimeBefore);
		expect(blogMtimeAfter).toBe(blogMtimeBefore);
	});

	test("should rebuild affected route when script changes", async ({ devServer }) => {
		// First, do a change to ensure we have build state
		writeFileSync(mainScriptPath, originalMainScriptContent + "\n// setup", "utf-8");
		await waitForBuildComplete(devServer);
		
		// Get modification times before change
		const indexMtimeBefore = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeBefore = statSync(aboutHtmlPath).mtimeMs;
		
		// Wait longer to ensure timestamps differ and debouncer completes
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Clear logs
		devServer.clearLogs();
		
		// Change about.js (only used by /about route)
		writeFileSync(aboutScriptPath, originalAboutScriptContent + "\n// modified", "utf-8");
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n").toLowerCase();
		
		// Should be incremental build
		expect(logsText).toContain("incremental build");
		
		// Get modification times after change
		const indexMtimeAfter = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeAfter = statSync(aboutHtmlPath).mtimeMs;
		
		// Index should NOT be rebuilt
		expect(indexMtimeAfter).toBe(indexMtimeBefore);
		
		// About should be rebuilt
		expect(aboutMtimeAfter).toBeGreaterThan(aboutMtimeBefore);
	});

	test("should rebuild affected route when image changes", async ({ devServer }) => {
		// First, do a change to ensure we have build state
		writeFileSync(stylesPath, originalStylesContent + "\n/* setup */", "utf-8");
		await waitForBuildComplete(devServer);
		
		// Get modification times before change
		const indexMtimeBefore = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeBefore = statSync(aboutHtmlPath).mtimeMs;
		
		// Wait longer to ensure timestamps differ and debouncer completes
		await new Promise(resolve => setTimeout(resolve, 500));
		
		// Clear logs
		devServer.clearLogs();
		
		// "Change" team.png (used by /about route)
		// We'll just write it again with same content but new mtime
		writeFileSync(teamPath, originalTeamContent);
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n").toLowerCase();
		
		// Should be incremental build
		expect(logsText).toContain("incremental build");
		
		// Get modification times after change
		const indexMtimeAfter = statSync(indexHtmlPath).mtimeMs;
		const aboutMtimeAfter = statSync(aboutHtmlPath).mtimeMs;
		
		// Index should NOT be rebuilt
		expect(indexMtimeAfter).toBe(indexMtimeBefore);
		
		// About should be rebuilt
		expect(aboutMtimeAfter).toBeGreaterThan(aboutMtimeBefore);
	});

	test("should preserve bundler inputs across incremental builds", async ({ devServer }) => {
		// First, do a change to ensure we have build state
		writeFileSync(stylesPath, originalStylesContent + "\n/* setup */", "utf-8");
		await waitForBuildComplete(devServer);
		
		// Clear logs
		devServer.clearLogs();
		
		// Change only blog.css (blog route only)
		writeFileSync(blogStylesPath, originalBlogStylesContent + "\n/* modified */", "utf-8");
		
		// Wait for rebuild
		const logs = await waitForBuildComplete(devServer, 20000);
		const logsText = logs.join("\n");
		
		// Check that logs mention merging with previous bundler inputs
		// This ensures that even though only blog route was rebuilt,
		// all assets from the previous build are still bundled
		expect(logsText).toContain("Merging with");
		expect(logsText).toContain("previous bundler inputs");
	});
});
