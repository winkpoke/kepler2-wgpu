pub mod handlers;
pub mod routes;
pub mod state;
mod ws;

pub use routes::create_router;
pub use state::ServerState;
pub use ws::WsMessage;

/// Default server port
pub const DEFAULT_PORT: u16 = 3000;

/// Environment variable name for server port
pub const PORT_ENV_VAR: &str = "KEPLER_SERVER_PORT";
