import { createTestWithFixture, expect } from "./test-utils";
import { prefetchScript } from "./utils";

const test = createTestWithFixture("prefetch-prerender");

test.describe("Prefetch", () => {
	test("should create prefetch via speculation rules on Chromium or link element elsewhere", async ({
		page,
		browserName,
		devServer,
	}) => {
		await page.goto(devServer.url);

		// Inject prefetch function
		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch
		await page.evaluate(() => {
			window.prefetch("/about/");
		});

		if (browserName === "chromium") {
			// Chromium: Should create a speculation rules script with prefetch
			const speculationScript = page.locator('script[type="speculationrules"]').first();
			const scriptContent = await speculationScript.textContent();
			expect(scriptContent).toBeTruthy();
			if (scriptContent) {
				const rules = JSON.parse(scriptContent);
				expect(rules.prefetch).toBeDefined();
				expect(rules.prefetch[0].urls).toContain("/about/");
			}
		} else {
			// Non-Chromium: If link prefetch is supported, assert link element; otherwise, ensure no speculation script
			const supportsPrefetch = await page.evaluate(() => {
				const link = document.createElement("link");
				// Some browsers may not support relList.supports('prefetch')
				return !!(link.relList && link.relList.supports && link.relList.supports("prefetch"));
			});

			if (supportsPrefetch) {
				const prefetchLink = page.locator('link[rel="prefetch"]').first();
				await expect(prefetchLink).toHaveAttribute("href", "/about/");
			} else {
				const speculationScripts = await page.locator('script[type="speculationrules"]').all();
				expect(speculationScripts.length).toBe(0);
			}
		}
	});

	test("should not prefetch same URL twice", async ({ page, browserName, devServer }) => {
		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch twice
		await page.evaluate(() => {
			window.prefetch("/about/");
			window.prefetch("/about/");
		});

		if (browserName === "chromium") {
			// Should only have one speculation rules script
			const speculationScripts = await page.locator('script[type="speculationrules"]').all();
			expect(speculationScripts.length).toBe(1);
			const scriptContent = await speculationScripts[0].textContent();
			if (scriptContent) {
				const rules = JSON.parse(scriptContent);
				expect(rules.prefetch).toBeDefined();
				expect(rules.prefetch[0].urls).toContain("/about/");
			}
		} else {
			// Non-Chromium: If link prefetch is supported, expect one link; otherwise, expect no speculation script
			const supportsPrefetch = await page.evaluate(() => {
				const link = document.createElement("link");
				return !!(link.relList && link.relList.supports && link.relList.supports("prefetch"));
			});

			if (supportsPrefetch) {
				const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
				expect(prefetchLinks.length).toBe(1);
			} else {
				const speculationScripts = await page.locator('script[type="speculationrules"]').all();
				expect(speculationScripts.length).toBe(0);
			}
		}
	});

	test("should not prefetch current page", async ({ page, browserName, devServer }) => {
		await page.goto(`${devServer.url}/about/`);

		await page.addScriptTag({ content: prefetchScript });

		// Try to prefetch current page
		await page.evaluate(() => {
			window.prefetch("/about/");
		});

		if (browserName === "chromium") {
			// Should not create any speculation rules script
			const speculationScripts = await page.locator('script[type="speculationrules"]').all();
			expect(speculationScripts.length).toBe(0);
		} else {
			// Should not create any link element
			const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
			expect(prefetchLinks.length).toBe(0);
		}
	});

	test("should not prefetch cross-origin URLs", async ({ page, browserName, devServer }) => {
		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Try to prefetch cross-origin URL
		await page.evaluate(() => {
			window.prefetch("https://example.com/about/");
		});

		if (browserName === "chromium") {
			// Should not create any speculation rules script
			const speculationScripts = await page.locator('script[type="speculationrules"]').all();
			expect(speculationScripts.length).toBe(0);
		} else {
			// Should not create any link element
			const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
			expect(prefetchLinks.length).toBe(0);
		}
	});

	test("should use correct eagerness level without prerender", async ({
		page,
		browserName,
		devServer,
	}) => {
		test.skip(browserName !== "chromium", "Speculation Rules only supported in Chromium");

		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch with custom eagerness but no prerender
		await page.evaluate(() => {
			window.prefetch("/about/", { eagerness: "moderate" });
		});

		const speculationScript = page.locator('script[type="speculationrules"]').first();
		const scriptContent = await speculationScript.textContent();

		if (scriptContent) {
			const rules = JSON.parse(scriptContent);
			expect(rules.prefetch[0].eagerness).toBe("moderate");
			expect(rules.prerender).toBeUndefined();
		}
	});
});
