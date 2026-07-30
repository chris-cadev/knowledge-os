pub mod citations;
pub mod factory;
pub mod mock;
pub mod ollama;
pub mod openai;
pub mod pipeline;
pub mod prompt;
pub mod status;

pub use factory::create_chat_provider;
pub use mock::MockChatAdapter;
pub use ollama::OllamaChatAdapter;
pub use openai::OpenAiChatAdapter;
