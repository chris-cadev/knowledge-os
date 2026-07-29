pub mod mock;
pub mod ollama;
pub mod openai;

pub use mock::MockChatAdapter;
pub use ollama::OllamaChatAdapter;
pub use openai::OpenAiChatAdapter;
