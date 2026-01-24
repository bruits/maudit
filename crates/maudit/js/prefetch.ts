const preloadedUrls = new Set<string>();

interface PreloadConfig {
	skipConnectionCheck?: boolean;
	/**
	 * Enable prerendering using Speculation Rules API if supported.
	 * Falls back to prefetch if not supported. (default: false)
	 */
	prerender?: boolean;
	/**
	 * Hint to the browser as to how eagerly it should prefetch/prerender.
	 * Only works when prerender is enabled and browser supports Speculation Rules API.
	 * (default: 'immediate')
	 *
	 * - 'immediate': Prerender as soon as possible
	 * - 'eager': Prerender eagerly but not immediately
	 * - 'moderate': Prerender with moderate eagerness
	 * - 'conservative': Prerender conservatively
	 */
	eagerness?: "immediate" | "eager" | "moderate" | "conservative";
}

export function prefetch(url: string, config?: PreloadConfig) {
	let urlObj: URL;
	try {
		urlObj = new URL(url, window.location.href);
		urlObj.hash = "";
	} catch {
		throw new Error(`Invalid URL provided to prefetch: ${url}`);
	}

	const skipConnectionCheck = config?.skipConnectionCheck ?? false;
	const shouldPrerender = config?.prerender ?? false;
	const eagerness = config?.eagerness ?? "immediate";

	if (!canPrefetchUrl(urlObj, skipConnectionCheck)) {
		return;
	}

	preloadedUrls.add(urlObj.href);

	// Use Speculation Rules API for prerendering if enabled and supported
	if (shouldPrerender && supportsSpeculationRules()) {
		appendSpeculationRules(urlObj.href, eagerness);
		return;
	}

	// Fallback to link prefetch
	const linkElement = document.createElement("link");
	const supportsPrefetch = linkElement.relList?.supports?.("prefetch");

	if (supportsPrefetch) {
		linkElement.rel = "prefetch";
		linkElement.href = url;
		document.head.appendChild(linkElement);
	}
}

function canPrefetchUrl(url: URL, skipConnectionCheck: boolean): boolean {
	return (
		navigator.onLine && // 1. Don't prefetch if the browser is offline (duh)
		(skipConnectionCheck || !hasLimitedBandwidth()) && // 2. Don't prefetch if the user has limited bandwidth, unless explicitely asked
		window.location.origin === url.origin && // 3. Don't prefetch cross-origin URLs
		!preloadedUrls.has(url.href) && // 4. Don't prefetch URLs we've already prefetched
		(window.location.pathname !== url.pathname || // 5. Don't prefetch the current page (different path or query string)
			window.location.search !== url.search)
	);
}

function hasLimitedBandwidth(): boolean {
	// Chrome thing
	// https://caniuse.com/?search=navigator.connection
	if ("connection" in navigator) {
		const networkInfo = (navigator as any).connection;
		return networkInfo.saveData || networkInfo.effectiveType.endsWith("2g");
	}

	return false;
}

function supportsSpeculationRules(): boolean {
	return HTMLScriptElement.supports && HTMLScriptElement.supports("speculationrules");
}

/**
 * Appends a <script type="speculationrules"> tag to prerender the URL.
 *
 * Note: Each URL needs its own script element - modifying an existing
 * script won't trigger a new prerender.
 */
function appendSpeculationRules(url: string, eagerness: NonNullable<PreloadConfig["eagerness"]>) {
	const script = document.createElement("script");
	script.type = "speculationrules";
	script.textContent = JSON.stringify({
		prerender: [
			{
				source: "list",
				urls: [url],
				eagerness,
			},
		],
		// Include prefetch as fallback if prerender fails
		// https://github.com/WICG/nav-speculation/issues/162#issuecomment-1977818473
		prefetch: [
			{
				source: "list",
				urls: [url],
				eagerness,
			},
		],
	});
	document.head.appendChild(script);
}