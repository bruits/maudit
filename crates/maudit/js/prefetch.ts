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
	 * Only works when browser supports Speculation Rules API.
	 * (default: 'immediate')
	 *
	 * - 'immediate': Prefetch/prerender as soon as possible
	 * - 'eager': Prefetch/prerender eagerly but not immediately
	 * - 'moderate': Prefetch/prerender with moderate eagerness
	 * - 'conservative': Prefetch/prerender conservatively
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

	// Calculate relative path once (pathname + search, no origin)
	const path = urlObj.pathname + urlObj.search;

	// Use Speculation Rules API when supported
	if (HTMLScriptElement.supports && HTMLScriptElement.supports("speculationrules")) {
		appendSpeculationRules(path, eagerness, shouldPrerender);
		return;
	}

	// Fallback to link prefetch for other browsers
	const linkElement = document.createElement("link");
	const supportsPrefetch = linkElement.relList?.supports?.("prefetch");

	if (supportsPrefetch) {
		linkElement.rel = "prefetch";
		linkElement.href = path;
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

/**
 * Appends a <script type="speculationrules"> tag to prefetch or prerender the URL.
 *
 * Note: Each URL needs its own script element - modifying an existing
 * script won't trigger a new prerender/prefetch.
 *
 * @param path - The relative path (pathname + search) to prefetch/prerender
 * @param eagerness - How eagerly the browser should prefetch/prerender
 * @param prerender - Whether to include a prerender rule
 */
function appendSpeculationRules(
	path: string,
	eagerness: NonNullable<PreloadConfig["eagerness"]>,
	prerender: boolean,
) {
	const script = document.createElement("script");
	script.type = "speculationrules";

	// We always want the prefetch, even if prerendering as a fallback
	const rules: any = {
		prefetch: [
			{
				source: "list",
				urls: [path],
				eagerness,
			},
		],
	};

	if (prerender) {
		rules.prerender = [
			{
				source: "list",
				urls: [path],
				eagerness,
			},
		];
	}

	script.textContent = JSON.stringify(rules);
	document.head.appendChild(script);
}
