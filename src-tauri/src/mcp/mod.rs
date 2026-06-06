// Embedded MCP (Model Context Protocol) server.
//
// While the app runs it hosts a small JSON-RPC 2.0 endpoint on localhost so an
// external LLM agent can query and control the application. This is a minimal,
// MCP-compatible HTTP transport implementing `initialize`, `tools/list` and
// `tools/call`; the protocol surface will be expanded as the feature matures.
//
// Security: the listener binds to 127.0.0.1 only (never a public interface) and
// shares the exact same database handle as the UI through an Arc<Mutex<...>>.

use crate::db::models::EventFilter;
use crate::db::models::NewLedgerEvent;
use crate::db::Database;
use crate::logging::Logger;
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// Spawn the server on a dedicated thread. Failure to bind is logged but never
// fatal: the app remains fully usable without the MCP endpoint.
pub fn start(database: Arc<Mutex<Database>>, logger: Arc<Logger>, port: u16)
{
	thread::spawn(move || {
		let address = format!("127.0.0.1:{port}");
		let server = match tiny_http::Server::http(&address)
		{
			Ok(server) => server,
			Err(error) =>
			{
				logger.debug_at("mcp", &format!("MCP server could not bind {address}: {error}"));
				return;
			}
		};

		logger.debug_at("mcp", &format!("MCP server listening on http://{address}"));

		// Handle requests one at a time; agent traffic is low volume.
		for mut request in server.incoming_requests()
		{
			// Pre-flight CORS so browser-based agent clients can connect locally.
			if request.method() == &tiny_http::Method::Options
			{
				let _ = request.respond(empty_cors_response());
				continue;
			}

			let mut body = String::new();
			if request.as_reader().read_to_string(&mut body).is_err()
			{
				let _ = request.respond(json_response(&error_envelope(Value::Null, -32700, "could not read request body")));
				continue;
			}

			let reply = dispatch(&database, &logger, &body);
			let _ = request.respond(json_response(&reply));
		}
	});
}

// Route a single JSON-RPC request to the matching handler.
fn dispatch(database: &Arc<Mutex<Database>>, logger: &Arc<Logger>, body: &str) -> Value
{
	// Parse the JSON-RPC envelope; malformed JSON is a parse error.
	let request: Value = match serde_json::from_str(body)
	{
		Ok(value) => value,
		Err(_) => return error_envelope(Value::Null, -32700, "invalid JSON"),
	};

	let id = request.get("id").cloned().unwrap_or(Value::Null);
	let method = request.get("method").and_then(|value| value.as_str()).unwrap_or("");
	let params = request.get("params").cloned().unwrap_or(Value::Null);

	logger.debug_at("mcp", &format!("MCP method {method}"));

	match method
	{
		"initialize" => result_envelope(id, initialize_result()),
		"tools/list" => result_envelope(id, json!({ "tools": tool_definitions() })),
		"tools/call" => call_tool(database, id, &params),
		"ping" => result_envelope(id, json!({})),
		_ => error_envelope(id, -32601, &format!("unknown method '{method}'")),
	}
}

// The MCP initialize handshake response.
fn initialize_result() -> Value
{
	json!({
		"protocolVersion": "2024-11-05",
		"capabilities": { "tools": {} },
		"serverInfo": {
			"name": "MyOpenUKTaxApp",
			"version": env!("CARGO_PKG_VERSION")
		}
	})
}

// Execute one tool call and wrap the outcome in MCP content.
fn call_tool(database: &Arc<Mutex<Database>>, id: Value, params: &Value) -> Value
{
	let name = params.get("name").and_then(|value| value.as_str()).unwrap_or("");
	let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

	// Each arm computes a serde_json value or an error string.
	let outcome: Result<Value, String> = run_tool(database, name, &arguments);

	match outcome
	{
		Ok(value) =>
		{
			let text = serde_json::to_string_pretty(&value).unwrap_or_default();
			result_envelope(id, json!({ "content": [ { "type": "text", "text": text } ] }))
		}
		Err(message) => error_envelope(id, -32000, &message),
	}
}

// Map a tool name + arguments to a database operation. Locks the shared database
// only for the duration of the call.
fn run_tool(database: &Arc<Mutex<Database>>, name: &str, arguments: &Value) -> Result<Value, String>
{
	// A poisoned lock is reported as a tool error rather than panicking.
	let mut db = database.lock().map_err(|_| "database lock poisoned".to_string())?;

	let optional_kind = arguments.get("kind").and_then(|value| value.as_str());

	match name
	{
		"list_subcategories" => to_value(db.list_subcategories(optional_kind)),
		"list_category_mappings" => to_value(db.list_mappings()),
		"list_hmrc_categories" => to_value(db.list_hmrc_categories(optional_kind)),
		"list_hmrc_submissions" => to_value(db.list_submissions()),

		"list_events" =>
		{
			let kind = required_str(arguments, "kind")?;
			let filter = EventFilter {
				date_from: arguments.get("date_from").and_then(|v| v.as_str()).map(str::to_string),
				date_to: arguments.get("date_to").and_then(|v| v.as_str()).map(str::to_string),
				search_term: arguments.get("search_term").and_then(|v| v.as_str()).map(str::to_string),
			};
			to_value(db.list_events(&kind, &filter))
		}

		"get_dashboard_summary" =>
		{
			let date_from = arguments.get("date_from").and_then(|v| v.as_str());
			let date_to = arguments.get("date_to").and_then(|v| v.as_str());
			to_value(db.dashboard_summary(date_from, date_to))
		}

		"create_subcategory" =>
		{
			let kind = required_str(arguments, "kind")?;
			let name = required_str(arguments, "name")?;
			let description = arguments.get("description").and_then(|v| v.as_str()).unwrap_or("");
			to_value(db.create_subcategory(&kind, &name, description))
		}

		"create_event" =>
		{
			let input = NewLedgerEvent {
				kind: required_str(arguments, "kind")?,
				event_date: required_str(arguments, "event_date")?,
				subcategory_id: arguments
					.get("subcategory_id")
					.and_then(|v| v.as_i64())
					.ok_or_else(|| "subcategory_id is required".to_string())?,
				amount_pence: arguments
					.get("amount_pence")
					.and_then(|v| v.as_i64())
					.ok_or_else(|| "amount_pence is required".to_string())?,
				note: arguments.get("note").and_then(|v| v.as_str()).unwrap_or("").to_string(),
			};
			to_value(db.create_event(&input))
		}

		_ => Err(format!("unknown tool '{name}'")),
	}
}

// Convert an AppResult into the tool Result<Value, String> shape.
fn to_value<T: serde::Serialize>(result: crate::error::AppResult<T>) -> Result<Value, String>
{
	match result
	{
		Ok(value) => serde_json::to_value(value).map_err(|error| error.to_string()),
		Err(error) => Err(error.to_string()),
	}
}

// Pull a required string argument or produce a helpful error.
fn required_str(arguments: &Value, key: &str) -> Result<String, String>
{
	arguments
		.get(key)
		.and_then(|value| value.as_str())
		.map(|value| value.to_string())
		.ok_or_else(|| format!("'{key}' is required"))
}

// The advertised tool catalogue with minimal JSON-Schema input descriptions.
fn tool_definitions() -> Value
{
	json!([
		{
			"name": "list_subcategories",
			"description": "List user subcategories, optionally filtered by kind (income|expense).",
			"inputSchema": { "type": "object", "properties": { "kind": { "type": "string" } } }
		},
		{
			"name": "list_events",
			"description": "List income or expense events with optional date range and search term.",
			"inputSchema": {
				"type": "object",
				"required": ["kind"],
				"properties": {
					"kind": { "type": "string" },
					"date_from": { "type": "string" },
					"date_to": { "type": "string" },
					"search_term": { "type": "string" }
				}
			}
		},
		{
			"name": "get_dashboard_summary",
			"description": "Return income/expense totals and a per-subcategory breakdown for a date window.",
			"inputSchema": {
				"type": "object",
				"properties": { "date_from": { "type": "string" }, "date_to": { "type": "string" } }
			}
		},
		{
			"name": "list_category_mappings",
			"description": "List the user's subcategory-to-HMRC-category mappings.",
			"inputSchema": { "type": "object", "properties": {} }
		},
		{
			"name": "list_hmrc_categories",
			"description": "List the fixed HMRC categories, optionally filtered by kind.",
			"inputSchema": { "type": "object", "properties": { "kind": { "type": "string" } } }
		},
		{
			"name": "list_hmrc_submissions",
			"description": "List the quarterly HMRC submission history.",
			"inputSchema": { "type": "object", "properties": {} }
		},
		{
			"name": "create_subcategory",
			"description": "Create a new subcategory.",
			"inputSchema": {
				"type": "object",
				"required": ["kind", "name"],
				"properties": {
					"kind": { "type": "string" },
					"name": { "type": "string" },
					"description": { "type": "string" }
				}
			}
		},
		{
			"name": "create_event",
			"description": "Create an income or expense event. Amount is in integer pence.",
			"inputSchema": {
				"type": "object",
				"required": ["kind", "event_date", "subcategory_id", "amount_pence"],
				"properties": {
					"kind": { "type": "string" },
					"event_date": { "type": "string" },
					"subcategory_id": { "type": "integer" },
					"amount_pence": { "type": "integer" },
					"note": { "type": "string" }
				}
			}
		}
	])
}

// ---- JSON-RPC envelope and HTTP helpers ----------------------------------

fn result_envelope(id: Value, result: Value) -> Value
{
	json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_envelope(id: Value, code: i64, message: &str) -> Value
{
	json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

// Build a JSON HTTP response with permissive local CORS headers.
fn json_response(value: &Value) -> tiny_http::Response<std::io::Cursor<Vec<u8>>>
{
	let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
	let mut response = tiny_http::Response::from_string(body);
	for header in cors_headers()
	{
		response.add_header(header);
	}
	if let Ok(header) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
	{
		response.add_header(header);
	}
	response
}

// A bare 204-style response for CORS pre-flight (OPTIONS) requests.
fn empty_cors_response() -> tiny_http::Response<std::io::Empty>
{
	let mut response = tiny_http::Response::empty(204);
	for header in cors_headers()
	{
		response.add_header(header);
	}
	response
}

// The shared CORS headers permitting local agent clients to connect.
fn cors_headers() -> Vec<tiny_http::Header>
{
	let definitions: [(&[u8], &[u8]); 3] = [
		(b"Access-Control-Allow-Origin", b"*"),
		(b"Access-Control-Allow-Methods", b"POST, OPTIONS"),
		(b"Access-Control-Allow-Headers", b"Content-Type"),
	];

	definitions
		.iter()
		.filter_map(|(name, value)| tiny_http::Header::from_bytes(*name, *value).ok())
		.collect()
}
