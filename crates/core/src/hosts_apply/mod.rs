pub mod aggregate;
pub mod cmd_runner;
pub mod elevation;
pub mod error;
pub mod history;
pub mod pipeline;
pub mod platform_write;
pub mod target;
pub mod write;

pub use cmd_runner::{
    clear as clear_cmd_history, cmd_history_path, delete_by_id as delete_cmd_history_item,
    load as load_cmd_history, CommandRunResult,
};
pub use error::ApplyError;
pub use history::{delete_by_id, list_history, ApplyHistoryItem};
pub use pipeline::ApplyPipeline;
pub use target::HostsTarget;
