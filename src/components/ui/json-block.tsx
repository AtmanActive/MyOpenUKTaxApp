// A scrollable monospaced JSON viewer. Accepts either a pre-formatted string
// (e.g. stored request/response JSON) or any value to pretty-print. Used by the
// HMRC Put history rows and every HMRC Get card so nothing HMRC returns is hidden.

export function JsonBlock(props: { title?: string; data: unknown })
{
	const text =
		typeof props.data === "string"
			? props.data
			: props.data === undefined
				? ""
				: JSON.stringify(props.data, null, 2);

	return (
		<div>
			{props.title ? (
				<p className="mb-1 text-xs font-semibold text-muted-foreground">{props.title}</p>
			) : null}
			<pre className="max-h-64 overflow-auto rounded-md bg-muted p-2 text-xs">{text || "—"}</pre>
		</div>
	);
}
