//! Network play in the application (task 88, `app/docs/network.md`): the follower on the
//! simulation thread, the lobby (joining a server, or hosting one and joining it), and the
//! server window.

pub mod lobby;
pub mod play;
pub mod server_window;
pub mod session;

#[cfg(test)]
mod tests;
