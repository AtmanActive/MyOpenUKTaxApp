// HMRC fraud-prevention header construction.
//
// HMRC requires a set of `Gov-Client-*` / `Gov-Vendor-*` headers on every MTD
// API call so they can perform fraud analysis. This builds the subset that a
// desktop application can populate reliably from the backend. Headers that
// require data only the WebView can supply (screen size, window size, local
// IPs, MAC addresses) are intentionally omitted here and should be added once
// the frontend collects them; the connection method is declared accordingly.
//
// Reference: https://developer.service.hmrc.gov.uk/guides/fraud-prevention/

use chrono::Local;

// Build the fraud-prevention headers as ordered (name, value) pairs. `device_id`
// is a stable per-installation identifier persisted in settings.
pub fn build(device_id: &str) -> Vec<(&'static str, String)>
{
	let mut headers: Vec<(&'static str, String)> = Vec::new();

	// A direct desktop application talking to HMRC on the user's behalf.
	headers.push(("Gov-Client-Connection-Method", "DESKTOP_APP_DIRECT".to_string()));

	// A stable identifier for this installation of the app.
	headers.push(("Gov-Client-Device-ID", device_id.to_string()));

	// The user's timezone, formatted as required, e.g. "UTC+01:00".
	headers.push(("Gov-Client-Timezone", local_timezone_header()));

	// Operating-system user identifier, declared with the "os" key.
	if let Ok(user_name) = std::env::var("USERNAME").or_else(|_| std::env::var("USER"))
	{
		headers.push((
			"Gov-Client-User-IDs",
			format!("os={}", urlencoding::encode(&user_name)),
		));
	}

	// Vendor identification: product name and version of this app.
	headers.push(("Gov-Vendor-Product-Name", "MyOpenUKTaxApp".to_string()));
	headers.push(("Gov-Vendor-Version", format!("MyOpenUKTaxApp={}", env!("CARGO_PKG_VERSION"))));

	// Basic platform user-agent string assembled from compile-time facts.
	headers.push((
		"Gov-Client-User-Agent",
		format!("os-family={}&os-version={}", std::env::consts::OS, std::env::consts::ARCH),
	));

	headers
}

// Format the current local UTC offset as HMRC expects, e.g. "UTC+00:00".
fn local_timezone_header() -> String
{
	let offset_seconds = Local::now().offset().local_minus_utc();
	let sign = if offset_seconds >= 0 { '+' } else { '-' };
	let absolute = offset_seconds.abs();
	let hours = absolute / 3600;
	let minutes = (absolute % 3600) / 60;
	format!("UTC{sign}{hours:02}:{minutes:02}")
}
