// Component modules for limit-tui

pub mod activity;
pub mod chat;
pub mod diff;
pub mod file_autocomplete;
pub mod input_queue;
pub mod pending_input;
pub mod progress;
pub mod prompt;

pub use activity::ActivityFeed;
pub use chat::{ChatView, Message, Role};
pub use diff::{parse_diff, DiffLine, DiffType, DiffView};
pub use file_autocomplete::{calculate_popup_area, FileAutocompleteWidget, FileMatchData};
pub use input_queue::{
    EscActionResult, InputQueueManager, InputState, LocalImageAttachment, MentionBinding,
    MessageId, PendingSteer, QueueError, TextElement, TextElementType, UserMessage,
    MAX_PENDING_STEERS, MAX_QUEUED_MESSAGES,
};
pub use pending_input::PendingInputPreview;
pub use progress::{ProgressBar, Spinner};
pub use prompt::{InputPrompt, InputResult, SelectPrompt, SelectResult};
