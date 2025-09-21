const ansiPattern = new RegExp(
	"(?:\\u001B\\][\\s\\S]*?(?:\\u0007|\\u001B\\u005C|\\u009C))|[\\u001B\\u009B][[\\]()#;?]*(?:\\d{1,4}(?:[;:]\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]",
	"g"
);

export function stripAnsi(str: string): string {
	return str.replace(ansiPattern, "");
}
