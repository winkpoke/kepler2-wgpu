pub mod handlers;
pub mod routes;
pub mod navcomputer;
pub mod state;
pub mod ai;
pub mod ai_task;
pub mod ai_model;
pub mod ai_handler;
mod ws;

pub use routes::create_router;
pub use state::ServerState;
pub use ws::WsMessage;

/// Default server port
pub const DEFAULT_PORT: u16 = 3000;

/// Environment variable name for server port
pub const PORT_ENV_VAR: &str = "KEPLER_SERVER_PORT";
