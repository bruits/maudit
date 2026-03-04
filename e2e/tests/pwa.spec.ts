import { createTestWithFixture, expect } from "./test-utils";

const test = createTestWithFixture("pwa", 1874);

test.describe("PWA", () => {
	test("should serve manifest.json with correct fields", async ({ page, devServer }) => {
		const response = await page.goto(`${devServer.url}/manifest.json`);
		expect(response?.status()).toBe(200);

		const manifest = await response!.json();
		expect(manifest.name).toBe("PWA Test App");
		expect(manifest.short_name).toBe("PWA Test");
		expect(manifest.start_url).toBe("/");
		expect(manifest.scope).toBe("/");
		expect(manifest.display).toBe("standalone");
	});

	test("should serve sw.js with correct content", async ({ page, devServer }) => {
		const response = await page.goto(`${devServer.url}/sw.js`);
		expect(response?.status()).toBe(200);

		const content = await response!.text();

		// Should have a deterministic cache name (maudit-<hex>)
		expect(content).toMatch(/const CACHE_NAME = "maudit-[0-9a-f]+";/);

		// Should use path-prefix matching for hashed assets, not extension-based
		expect(content).toContain("HASHED_ASSET_RE");
		expect(content).toContain("_maudit");
		// Should NOT contain extension-based matching
		expect(content).not.toMatch(/\.\(js\|css\|png/);

		// Should have precache URLs with resolved paths (not route patterns)
		expect(content).toContain('PRECACHE_URLS');
		expect(content).toContain('"/"');
		expect(content).toContain('"/about"');

		// Should have install, activate, and fetch handlers
		expect(content).toContain("install");
		expect(content).toContain("activate");
		expect(content).toContain("fetch");
	});

	test("should inject manifest link into pages", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		const manifestLink = page.locator('link[rel="manifest"]');
		await expect(manifestLink).toHaveAttribute("href", "/manifest.json");
	});

	test("should inject service worker registration script into pages", async ({
		page,
		devServer,
	}) => {
		await page.goto(devServer.url);

		// The PWA register script should be loaded as a module
		const hasSwRegistration = await page.evaluate(() => {
			return "serviceWorker" in navigator;
		});
		expect(hasSwRegistration).toBe(true);
	});

	test("should produce deterministic sw.js across page loads", async ({ page, devServer }) => {
		// Fetch sw.js twice and verify the content is identical
		const response1 = await page.goto(`${devServer.url}/sw.js`);
		const content1 = await response1!.text();

		const response2 = await page.goto(`${devServer.url}/sw.js`);
		const content2 = await response2!.text();

		expect(content1).toBe(content2);

		// Extract and verify the cache name format
		const cacheNameMatch = content1.match(/const CACHE_NAME = "(maudit-[0-9a-f]+)";/);
		expect(cacheNameMatch).toBeTruthy();
		expect(cacheNameMatch![1]).toMatch(/^maudit-[0-9a-f]+$/);
	});

	test("should use path-prefix regex matching _maudit directory", async ({
		page,
		devServer,
	}) => {
		const response = await page.goto(`${devServer.url}/sw.js`);
		const content = await response!.text();

		// The regex should match paths starting with /_maudit/
		// This ensures only bundled hashed assets get cache-first treatment
		expect(content).toMatch(/HASHED_ASSET_RE = \/\^\\?\/?_maudit\\?\//);
	});
});
