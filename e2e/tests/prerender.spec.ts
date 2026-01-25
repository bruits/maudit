import { test, expect } from "./test-utils";
import { prefetchScript } from "./utils";

test.describe("Prefetch - Speculation Rules (Prerender)", () => {
	test("should create speculation rules script when prerender is enabled", async ({
		page,
		browserName,
		devServer,
	}) => {
		// Skip on non-Chromium browsers (Speculation Rules only supported in Chrome/Edge)
		test.skip(browserName !== "chromium", "Speculation Rules only supported in Chromium");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch with prerender
		await page.evaluate(() => {
			window.prefetch("/about/", { prerender: true });
		});

		// Check that a speculation rules script was created
		const speculationScript = page.locator('script[type="speculationrules"]').first();
		expect(speculationScript).toBeDefined();

		// Check script content
		const scriptContent = await speculationScript.textContent();
		expect(scriptContent).toBeTruthy();

		if (scriptContent) {
			const rules = JSON.parse(scriptContent);
			expect(rules.prerender).toBeDefined();
			expect(rules.prerender[0].urls).toContain("/about/");
			expect(rules.prefetch).toBeDefined(); // Fallback
			expect(rules.prefetch[0].urls).toContain("/about/");
		}
	});

	test("should use correct eagerness level", async ({ page, browserName, devServer }) => {
		test.skip(browserName !== "chromium", "Speculation Rules only supported in Chromium");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch with custom eagerness
		await page.evaluate(() => {
			window.prefetch("/about/", { prerender: true, eagerness: "conservative" });
		});

		const speculationScript = page.locator('script[type="speculationrules"]').first();
		const scriptContent = await speculationScript.textContent();

		if (scriptContent) {
			const rules = JSON.parse(scriptContent);
			expect(rules.prerender[0].eagerness).toBe("conservative");
			expect(rules.prefetch[0].eagerness).toBe("conservative");
		}
	});

	test("should fallback to link prefetch when speculation rules not supported", async ({
		page,
		browserName,
		devServer,
	}) => {
		// Run this test on Firefox/Safari where Speculation Rules is not supported
		test.skip(browserName === "chromium", "Testing fallback behavior on non-Chromium browsers");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch with prerender (should fallback to link)
		await page.evaluate(() => {
			window.prefetch("/about/", { prerender: true });
		});

		// Should create link element instead
		const prefetchLink = page.locator('link[rel="prefetch"]').first();
		await expect(prefetchLink).toHaveAttribute("href", "/about/");

		// Should NOT create speculation rules script
		const speculationScripts = await page.locator('script[type="speculationrules"]').all();
		expect(speculationScripts.length).toBe(0);
	});

	test("should not prerender same URL twice", async ({ page, browserName, devServer }) => {
		test.skip(browserName !== "chromium", "Speculation Rules only supported in Chromium");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch with prerender twice
		await page.evaluate(() => {
			window.prefetch("/about/", { prerender: true });
			window.prefetch("/about/", { prerender: true });
		});

		// Should only have one speculation rules script
		const speculationScripts = await page.locator('script[type="speculationrules"]').all();
		expect(speculationScripts.length).toBe(1);
	});

	test("should create separate scripts for different URLs", async ({ page, browserName, devServer }) => {
		test.skip(browserName !== "chromium", "Speculation Rules only supported in Chromium");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Prerender multiple URLs
		await page.evaluate(() => {
			window.prefetch("/about/", { prerender: true });
			window.prefetch("/contact/", { prerender: true });
			window.prefetch("/blog/", { prerender: true });
		});

		// Should have three separate scripts (one per URL)
		const speculationScripts = await page.locator('script[type="speculationrules"]').all();
		expect(speculationScripts.length).toBe(3);
	});
});
