const ansiPattern = new RegExp(
	// oxlint-disable-next-line no-control-regex
	"(?:\\u001B\\][\\s\\S]*?(?:\\u0007|\\u001B\\u005C|\\u009C))|[\\u001B\\u009B][[\\]()#;?]*(?:\\d{1,4}(?:[;:]\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]",
	"g",
);

export function stripAnsi(str: string): string {
	return str.replace(ansiPattern, "");
}

export function log(...args: unknown[]) {
	mauditMessage("log", args);
}

export function warn(...args: unknown[]) {
	mauditMessage("warn", args);
}

export function error(...args: unknown[]) {
	mauditMessage("error", args);
}

function mauditMessage(level: "log" | "warn" | "error", message: unknown[]) {
	console[level](
		"%cMaudit",
		"background: #ba1f33; color: white; padding-inline: 4px; border-radius: 2px; font-family: serif;",
		...message.map((m) => (typeof m === "string" ? stripAnsi(m) : JSON.stringify(m, null, 2))),
	);
}
