// Formatting and parsing helpers for money and dates.
//
// Money is stored as integer pence everywhere; these helpers convert to/from the
// GBP decimal strings shown in the UI without ever using floating point for
// storage. GBP and date formatting use the en-GB locale.

const gbp_formatter = new Intl.NumberFormat("en-GB", {
	style: "currency",
	currency: "GBP",
});

// Render integer pence as e.g. "£1,234.56".
export function format_money_from_pence(pence: number): string
{
	const safe_pence = Number.isFinite(pence) ? pence : 0;
	return gbp_formatter.format(safe_pence / 100);
}

// Parse a user-entered pounds string (allowing "£" and thousands separators)
// into integer pence, or null if it is not a valid non-negative number.
export function parse_pounds_to_pence(text: string): number | null
{
	const cleaned = text.replace(/[£,\s]/g, "");
	if (cleaned === "")
	{
		return null;
	}
	const value = Number(cleaned);
	if (!Number.isFinite(value) || value < 0)
	{
		return null;
	}
	return Math.round(value * 100);
}

// Today's date as an ISO YYYY-MM-DD string for date inputs.
export function today_iso(): string
{
	const now = new Date();
	const year = now.getFullYear();
	const month = String(now.getMonth() + 1).padStart(2, "0");
	const day = String(now.getDate()).padStart(2, "0");
	return `${year}-${month}-${day}`;
}

// Render an RFC3339/ISO timestamp as a readable local date-time, leaving an
// already-short value untouched if it does not parse.
export function format_datetime(iso: string): string
{
	if (!iso)
	{
		return "";
	}
	const parsed = new Date(iso);
	return Number.isNaN(parsed.getTime()) ? iso : parsed.toLocaleString("en-GB");
}
