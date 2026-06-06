// Small shared utilities.
//
// `random_hex_token` produces an opaque identifier used for the per-installation
// device id and the OAuth `state` value. It is intentionally NOT presented as a
// cryptographic random source: it mixes the high-resolution clock, the process
// id and a monotonic counter. That is adequate for an installation identifier
// and a CSRF nonce on a single-user desktop app; security-critical randomness
// should use a vetted crate if ever needed.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

// Monotonic counter so two calls in the same nanosecond still differ.
static CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

// Produce a hex token of roughly `hex_length` characters.
pub fn random_hex_token(hex_length: usize) -> String
{
	let mut output = String::with_capacity(hex_length);

	// Seed material that changes on every call and across processes.
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_nanos())
		.unwrap_or(0);
	let process_id = std::process::id();

	let mut state = nanos as u64 ^ ((process_id as u64) << 17);

	// Generate 16 hex chars per round by hashing the evolving state.
	while output.len() < hex_length
	{
		let counter = CALL_COUNTER.fetch_add(1, Ordering::Relaxed);

		let mut hasher = DefaultHasher::new();
		state.hash(&mut hasher);
		counter.hash(&mut hasher);
		nanos.hash(&mut hasher);
		let hashed = hasher.finish();

		output.push_str(&format!("{hashed:016x}"));
		state = state.wrapping_add(hashed).rotate_left(13);
	}

	output.truncate(hex_length);
	output
}
