# IP-012: Pluggable OCR Backend with Image Blobs as Canonical

**Status:** Draft
**ADR(s):** [ADR-0026](../../architecture/adrs/adr-0026.md) (Pluggable OCR Backend with Image Blobs as Canonical and OCR Text as Derived)
**PRD(s):** [PRD-0007](../prds/prd-0007-knowledge-chat-and-universal-import.md) (F1.I OCR for Images and Scanned Documents)
**Estimated effort:** ~3 days

---

## Context

ADR-0026 establishes OCR as a derivation-layer activity. Image blobs are canonical (stored via `BinaryContent`); OCR text is derived (stored as `Content` component on the same entity). The `OcrBackend` trait defines the backend contract. Built-in adapters for Tesseract, Ollama vision, OpenAI-compatible vision, and Mock are required.

**Current state:**
- `core/knowledge-core/src/ports/ai.rs` defines `AiAdapter` for embeddings. The new `OcrBackend` trait is a parallel port for OCR.
- `core/knowledge-core/src/features/component/mod.rs` defines `ComponentType::BinaryContent` (already used for PDF and image storage) and `ComponentType::Content` (for extracted text).
- `core/knowledge-import/Cargo.toml` already includes `reqwest`, `serde`, `serde_json`, `tokio`. New dependencies for OCR are needed: `tesseract-rs`, `image`, and additional `reqwest` features.
- The `BinaryContent` component payload is `{ reference, mime_type, size }` (per ADR-0011). The image bytes are stored in object storage, not in the database.
- No OCR pipeline exists today. The `PdfImporter` (in `core/knowledge-import/src/features/importer/pdf.rs`) extracts text directly from PDF text streams. Scanned PDFs are not handled.
- The `MarkdownImporter` and `PdfImporter` do not extract embedded images.

**Dependencies:**
- IP-009 (ChatCompletion trait) — independent, but follows the same adapter pattern
- IP-010 (Conversation entities) — independent
- IP-013 (Universal Import) — partially depends on this plan (image extraction from office files)

This plan can be developed in parallel with IP-009, IP-010, and IP-011. IP-013 D8 (image extraction from office files) depends on this plan's `OcrBackend` trait.

---

## Deliverables

### D1: OcrBackend Trait and Image/OcrResult Types

**Purpose:** Define the `OcrBackend` port trait in `knowledge-core` with typed input/output.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-core/src/ports/ocr.rs` | Create | `OcrBackend` trait, `ImageInput`, `OcrResult`, `TextBlock`, `BoundingBox`, `OcrError` |
| `core/knowledge-core/src/ports/mod.rs` | Modify | Add `pub mod ocr; pub use ocr::*;` |

**New types:**

```rust
// core/knowledge-core/src/ports/ocr.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInput {
    pub bytes: Vec<u8>,
    pub mime_type: String,  // "image/png", "image/jpeg", "image/gif", "image/bmp"
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub bbox: BoundingBox,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f64,
    pub blocks: Vec<TextBlock>,
    pub model: String,  // e.g., "tesseract-5", "deepseek-ocr", "gpt-4o-vision"
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("image decode error: {0}")]
    ImageDecode(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

#[async_trait]
pub trait OcrBackend: Send + Sync {
    /// Recognize text from an image.
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError>;

    /// The name of this backend (e.g., "tesseract", "ollama", "api", "mock")
    fn name(&self) -> &str;

    /// Whether this backend requires a network connection.
    fn requires_network(&self) -> bool;
}
```

**Verification:**
- `cargo check -p knowledge-core` compiles
- `cargo test -p knowledge-core` passes (no regressions)

**Exit criteria:** Trait and types compile, no behavioral change to existing ports.

---

### D2: MockOcrBackend

**Purpose:** Implement `OcrBackend` for the Mock provider — deterministic OCR results for testing.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/ocr/mod.rs` | Create | `ocr` module |
| `core/knowledge-derivation/src/features/ocr/mock.rs` | Create | `MockOcrBackend` implementation |

**Implementation:**

```rust
// core/knowledge-derivation/src/features/ocr/mock.rs
use async_trait::async_trait;
use knowledge_core::ports::ocr::*;

pub struct MockOcrBackend {
    /// Pre-canned text to return for any image.
    canned_text: String,
    /// Confidence score to return.
    confidence: f64,
}

impl MockOcrBackend {
    pub fn new() -> Self {
        Self {
            canned_text: "Mock OCR result".to_string(),
            confidence: 0.95,
        }
    }

    /// Configure canned text for a specific test.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.canned_text = text.into();
        self
    }
}

impl Default for MockOcrBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl OcrBackend for MockOcrBackend {
    async fn recognize(&self, _image: &ImageInput) -> Result<OcrResult, OcrError> {
        Ok(OcrResult {
            text: self.canned_text.clone(),
            confidence: self.confidence,
            blocks: vec![TextBlock {
                text: self.canned_text.clone(),
                bbox: BoundingBox { x: 0, y: 0, width: 100, height: 20 },
                confidence: self.confidence,
            }],
            model: "mock-ocr".to_string(),
        })
    }

    fn name(&self) -> &str { "mock" }
    fn requires_network(&self) -> bool { false }
}
```

**Tests:**
- `mock_recognize_returns_canned_text` — `recognize()` returns configured text
- `mock_name_is_mock` — `name()` returns `"mock"`
- `mock_no_network` — `requires_network()` returns `false`
- `mock_zero_byte_image` — empty image bytes still returns result (no error)

**Verification:**
- `cargo test -p knowledge-derivation` passes with 4 new tests

**Exit criteria:** `MockOcrBackend` is testable and deterministic.

---

### D3: TesseractOcrBackend

**Purpose:** Implement `OcrBackend` for Tesseract using the `tesseract-rs` crate.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/ocr/tesseract.rs` | Create | `TesseractOcrBackend` |
| `core/knowledge-derivation/src/features/ocr/mod.rs` | Modify | Re-export `TesseractOcrBackend` |
| `core/knowledge-derivation/Cargo.toml` | Modify | Add `tesseract-rs = "0.3"` and `image = "0.25"` |

**Implementation outline:**

```rust
// core/knowledge-derivation/src/features/ocr/tesseract.rs
use async_trait::async_trait;
use knowledge_core::ports::ocr::*;
use std::io::Cursor;

pub struct TesseractOcrBackend {
    /// Language code, default "eng". Multiple languages can be specified as "eng+fra".
    language: String,
}

impl TesseractOcrBackend {
    pub fn new() -> Self {
        Self { language: "eng".to_string() }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }
}

impl Default for TesseractOcrBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl OcrBackend for TesseractOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        // 1. Decode image to grayscale using `image` crate
        let img = image::load_from_memory(&image.bytes)
            .map_err(|e| OcrError::ImageDecode(e.to_string()))?;
        let gray = img.to_luma8();

        // 2. Wrap in Cursor for tesseract-rs
        let mut cursor = Cursor::new(gray.into_raw());

        // 3. Call tesseract (synchronous; run in blocking task)
        let lang = self.language.clone();
        let text = tokio::task::spawn_blocking(move || {
            let mut tess = tesseract::Tesseract::new(None, Some(&lang))
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            tess.set_image_from_mem(&cursor.get_ref())
                .map_err(|e| OcrError::Provider(e.to_string()))?;
            tess.get_text()
                .map_err(|e| OcrError::Provider(e.to_string()))
        }).await
        .map_err(|e| OcrError::Provider(format!("join error: {}", e)))??;

        Ok(OcrResult {
            text: text.trim().to_string(),
            confidence: 0.85,  // Tesseract doesn't easily expose per-image confidence
            blocks: vec![],    // Bounding boxes not extracted in initial version
            model: "tesseract-5".to_string(),
        })
    }

    fn name(&self) -> &str { "tesseract" }
    fn requires_network(&self) -> bool { false }
}
```

**Tests** (use a small test PNG generated in-test):
- `tesseract_recognizes_known_text` — generate a test image with known text, verify result contains it
- `tesseract_handles_unsupported_format` — invalid image bytes return `OcrError::ImageDecode`
- `tesseract_default_language` — default language is "eng"
- `tesseract_custom_language` — `with_language("fra")` is reflected in behavior (no error)
- `tesseract_no_network` — `requires_network()` returns `false`

**Verification:**
- `cargo test -p knowledge-derivation` passes with 5 new tests
- The `tesseract-rs` build with `build-tesseract` feature compiles the bundled Tesseract (no system dependency)

**Exit criteria:** `TesseractOcrBackend` is functional with bundled Tesseract.

---

### D4: OllamaOcrBackend and ApiOcrBackend

**Purpose:** Implement `OcrBackend` for Ollama vision models and OpenAI-compatible vision endpoints.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/ocr/ollama.rs` | Create | `OllamaOcrBackend` |
| `core/knowledge-derivation/src/features/ocr/api.rs` | Create | `ApiOcrBackend` for OpenAI-compatible vision |
| `core/knowledge-derivation/src/features/ocr/mod.rs` | Modify | Re-export both backends |
| `core/knowledge-derivation/Cargo.toml` | Modify | Add `base64 = "0.22"` for image encoding |

**Ollama implementation outline:**

```rust
// core/knowledge-derivation/src/features/ocr/ollama.rs
pub struct OllamaOcrBackend {
    client: reqwest::Client,
    model: String,
    endpoint: String,
}

impl OllamaOcrBackend {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            model: model.into(),
            endpoint: "http://localhost:11434".to_string(),
        }
    }
}

#[async_trait]
impl OcrBackend for OllamaOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        // POST {endpoint}/api/generate with:
        //   {
        //     "model": "deepseek-ocr",
        //     "prompt": "Extract all text from this image. Return only the text.",
        //     "images": ["<base64>"],
        //     "stream": false
        //   }
        // Response: { "response": "<extracted text>" }
        todo!()
    }
    fn name(&self) -> &str { "ollama" }
    fn requires_network(&self) -> bool { true }
}
```

**API implementation outline (OpenAI-compatible):**

```rust
// core/knowledge-derivation/src/features/ocr/api.rs
pub struct ApiOcrBackend {
    client: reqwest::Client,
    model: String,
    api_key: String,
    base_url: String,
}

impl ApiOcrBackend {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("HTTP client"),
            model,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl OcrBackend for ApiOcrBackend {
    async fn recognize(&self, image: &ImageInput) -> Result<OcrResult, OcrError> {
        // POST {base_url}/chat/completions with vision-format messages:
        //   {
        //     "model": "gpt-4o",
        //     "messages": [{
        //       "role": "user",
        //       "content": [
        //         { "type": "text", "text": "Extract all text from this image." },
        //         { "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }
        //       ]
        //     }]
        //   }
        todo!()
    }
    fn name(&self) -> &str { "api" }
    fn requires_network(&self) -> bool { true }
}
```

**Tests** (use `httpmock` for in-process HTTP mock):
- `ollama_recognize_sends_base64_image` — verify request body contains base64-encoded image
- `ollama_recognize_parses_response` — mock returns `{"response": "Hello world"}`, verify `OcrResult.text == "Hello world"`
- `ollama_with_custom_endpoint` — custom endpoint is used
- `api_recognize_sends_vision_format` — verify chat completions request format
- `api_recognize_with_lm_studio` — custom `base_url` like `http://localhost:1234/v1` works
- `api_recognize_maps_400_to_provider_error`
- `api_recognize_maps_network_error`

**Verification:**
- `cargo test -p knowledge-derivation` passes with 7 new tests

**Exit criteria:** Both backends implement the trait and are tested.

---

### D5: OCR Factory and OCR Pipeline

**Purpose:** Implement `create_ocr_backend()` factory and the `OcrPipeline` that processes image imports.

**Files:**

| File | Action | Description |
|------|--------|-------------|
| `core/knowledge-derivation/src/features/ocr/factory.rs` | Create | `create_ocr_backend()` factory |
| `core/knowledge-derivation/src/features/ocr/pipeline.rs` | Create | `OcrPipeline` for processing images and updating Content |
| `core/knowledge-derivation/src/features/ocr/mod.rs` | Modify | Re-export factory and pipeline |

**Factory:**

```rust
// core/knowledge-derivation/src/features/ocr/factory.rs
pub fn create_ocr_backend(config: &str) -> Result<Box<dyn OcrBackend>, OcrError> {
    // "mock" or "mock://" → MockOcrBackend
    if config == "mock" || config.starts_with("mock://") {
        return Ok(Box::new(super::mock::MockOcrBackend::default()));
    }
    // "tesseract" or "tesseract://LANG" → TesseractOcrBackend
    if config == "tesseract" || config.starts_with("tesseract://") {
        let lang = config.strip_prefix("tesseract://").unwrap_or("eng");
        return Ok(Box::new(super::tesseract::TesseractOcrBackend::new()
            .with_language(lang)));
    }
    // "ollama://MODEL" or "ollama://MODEL?url=URL" → OllamaOcrBackend
    if let Some(rest) = config.strip_prefix("ollama://") {
        // ... parse model and url
        return Ok(Box::new(super::ollama::OllamaOcrBackend::new(model)));
    }
    // "api://MODEL?api_key=KEY&base_url=URL" → ApiOcrBackend
    if let Some(rest) = config.strip_prefix("api://") {
        // ... parse model, api_key, base_url
        return Ok(Box::new(super::api::ApiOcrBackend::new(api_key, model)));
    }
    // Default: mock
    Ok(Box::new(super::mock::MockOcrBackend::default()))
}
```

**OcrPipeline:**

```rust
// core/knowledge-derivation/src/features/ocr/pipeline.rs
pub struct OcrPipeline {
    backend: Arc<dyn OcrBackend>,
    component_repo: Arc<dyn ComponentRepository>,
}

impl OcrPipeline {
    pub fn new(backend: Arc<dyn OcrBackend>, component_repo: Arc<dyn ComponentRepository>) -> Self {
        Self { backend, component_repo }
    }

    /// Process an image: read from BinaryContent, run OCR, update Content component.
    pub async fn process_image(
        &self,
        entity_id: Uuid,
        image_bytes: Vec<u8>,
        mime_type: String,
    ) -> Result<OcrResult, OcrError> {
        // 1. Decode image to get dimensions
        let img = image::load_from_memory(&image_bytes)
            .map_err(|e| OcrError::ImageDecode(e.to_string()))?;
        let input = ImageInput {
            bytes: image_bytes,
            mime_type,
            width: img.width(),
            height: img.height(),
        };

        // 2. Run OCR
        let result = self.backend.recognize(&input).await?;

        // 3. Update Content component on the entity
        let content = Component::new(
            entity_id,
            ComponentType::Content,
            serde_json::json!({ "markdown": result.text }),
        );
        // ... check if Content exists, update or add
        todo!()
    }
}
```

**Tests:**
- `factory_creates_mock_for_mock_scheme`
- `factory_creates_tesseract_default`
- `factory_creates_tesseract_with_custom_language`
- `factory_creates_ollama_for_ollama_scheme`
- `factory_creates_api_for_api_scheme`
- `factory_defaults_to_mock`
- `pipeline_processes_image_and_updates_content`

**Verification:**
- `cargo test -p knowledge-derivation` passes with 7 new tests

**Exit criteria:** Factory handles all schemes, pipeline updates Content component.

---

## Execution Order

```
D1 (trait/types) -> D2 (Mock) -> D3 (Tesseract) -> D4 (Ollama + API) -> D5 (factory + pipeline)
```

D1 is type definitions. D2 is test-only. D3 and D4 are real backends. D5 wires the factory and pipeline.

---

## Verification Strategy

| Level | Command | Coverage |
|-------|---------|----------|
| Unit | `cargo test -p knowledge-core` | Trait compilation |
| Unit | `cargo test -p knowledge-derivation` | All 4 backends + factory + pipeline |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` | Code quality |

---

## Exit Criteria

- [ ] `OcrBackend` trait in `core/knowledge-core/src/ports/ocr.rs`
- [ ] `MockOcrBackend` (4 tests)
- [ ] `TesseractOcrBackend` with bundled Tesseract (5 tests)
- [ ] `OllamaOcrBackend` for vision models (3 tests)
- [ ] `ApiOcrBackend` for OpenAI-compatible vision (4 tests)
- [ ] `create_ocr_backend()` factory (5 tests)
- [ ] `OcrPipeline` that updates Content component (1 test)
- [ ] All existing tests pass (no regressions)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] ADR-0026 updated with Implementation Notes

---

## Impact Analysis

### Structural Changes and Consumers

| Change | Direct Consumers | Transitive Consumers |
|--------|------------------|---------------------|
| `OcrBackend` trait (new) | OCR backends, `OcrPipeline` | `OcrPipeline` consumers (image import in IP-013 D8) |
| `OcrPipeline` (new) | Standalone image import in `knowledge-import` | Office file imports (IP-013 D8) |
| `tesseract-rs` dependency in `knowledge-derivation` | `TesseractOcrBackend` | Workspace-wide rebuild |
| `image` crate dependency in `knowledge-derivation` | Image decoding (used by Tesseract and other backends) | `MarkdownImporter` and `PdfImporter` can use for image extraction |

### Risk Surface

1. **Tesseract compilation time:** Building Tesseract from source (via the `build-tesseract` feature) adds 5-10 minutes to the first build. **Mitigation:** The `build-tesseract` feature is opt-in. CI builds use the feature; developer machines can use system Tesseract.

2. **OCR accuracy variance:** Tesseract is less accurate than Ollama vision or GPT-4o vision. **Mitigation:** Default backend is Tesseract for speed. The user can switch to Ollama or API for higher accuracy.

3. **Image decoding failures:** Some image formats (e.g., WebP, HEIC) are not supported by the `image` crate without additional features. **Mitigation:** Supported formats are PNG, JPEG, GIF, BMP. Unsupported formats return `OcrError::UnsupportedFormat` with a clear message.

4. **Backend configuration drift:** The factory parsing has many edge cases. **Mitigation:** The factory is tested with 5+ configuration strings. Misconfiguration returns a clear error.

5. **OCR is async but Tesseract is sync:** Tesseract runs in a blocking task via `tokio::task::spawn_blocking`. **Mitigation:** The async signature is preserved. The blocking task is bounded by the `tokio` runtime's blocking thread pool.

---

## Implementation Notes

*(Filled in during/after implementation — records deviations, discoveries, decisions made during coding)*
