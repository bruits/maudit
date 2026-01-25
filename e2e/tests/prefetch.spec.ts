import { test, expect } from "./test-utils";
import { prefetchScript } from "./utils";

test.describe("Prefetch", () => {
	test("should create link element for prefetch", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		// Inject prefetch function
		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch
		await page.evaluate(() => {
			window.prefetch("/about/");
		});

		// Check that a link element was created
		const prefetchLink = page.locator('link[rel="prefetch"]').first();
		await expect(prefetchLink).toHaveAttribute("href", "/about/");
	});

	test("should not prefetch same URL twice", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Call prefetch twice
		await page.evaluate(() => {
			window.prefetch("/about/");
			window.prefetch("/about/");
		});

		// Should only have one link element
		const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
		expect(prefetchLinks.length).toBe(1);
	});

	test("should not prefetch current page", async ({ page, devServer }) => {
		await page.goto(`${devServer.url}/about/`);

		await page.addScriptTag({ content: prefetchScript });

		// Try to prefetch current page
		await page.evaluate(() => {
			window.prefetch("/about/");
		});

		// Should not create any link element
		const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
		expect(prefetchLinks.length).toBe(0);
	});

	test("should not prefetch cross-origin URLs", async ({ page, devServer }) => {
		await page.goto(devServer.url);

		await page.addScriptTag({ content: prefetchScript });

		// Try to prefetch cross-origin URL
		await page.evaluate(() => {
			window.prefetch("https://example.com/about/");
		});

		// Should not create any link element
		const prefetchLinks = await page.locator('link[rel="prefetch"]').all();
		expect(prefetchLinks.length).toBe(0);
	});
});
