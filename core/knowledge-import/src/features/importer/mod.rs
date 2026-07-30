pub mod adapter;
pub mod calendar_contact;
pub mod clipboard;
pub mod database;
pub mod directory;
pub mod docx;
pub mod email;
pub mod html;
pub mod ignore_config;
pub mod image;
pub mod iwork;
pub mod legacy_office;
pub mod magic_bytes;
pub mod markdown;
pub mod mbox;
pub mod note_apps;
pub mod obsidian;
pub mod opendocument;
pub mod pdf;
pub mod plugins;
pub mod pptx;
pub mod structured;
pub mod undo;
pub mod url;
pub mod watch;
pub mod xlsm;
pub mod xlsx;

pub use adapter::{CrossReference, ImportAdapter, ImportError, ImportResult};
pub use calendar_contact::{IcsImporter, VcfImporter};
pub use clipboard::ClipboardImporter;
pub use database::{MySqlDatabaseSource, PostgresDatabaseSource, SqliteDatabaseSource};
pub use directory::DirectoryImporter;
pub use docx::DocxImporter;
pub use email::{EmlImporter, MsgImporter};
pub use html::HtmlImporter;
pub use image::ImageImporter;
pub use iwork::{KeynoteImporter, NumbersImporter, PagesImporter};
pub use legacy_office::{DocImporter, PpsImporter, PptImporter, XlsImporter};
pub use magic_bytes::{detect_format, DetectedFormat};
pub use markdown::MarkdownImporter;
pub use mbox::MboxImporter;
pub use note_apps::{EnexImporter, NotionJsonImporter, OpmlImporter};
pub use obsidian::ObsidianVaultImporter;
pub use opendocument::{
    OdgImporter, OdpImporter, OdsImporter, OdtImporter, OtpImporter, OtsImporter, OttImporter,
};
pub use pdf::PdfImporter;
pub use plugins::{
    html_plugin, image_plugin, markdown_plugin, pdf_plugin, url_plugin, PluginAdapter,
};
pub use pptx::PptxImporter;
pub use structured::{CsvImporter, JsonImporter, XmlImporter, YamlImporter};
pub use undo::{get_import_history, get_previous_hashes, record_import, undo_last_import};
pub use url::UrlImporter;
pub use watch::DirectoryWatcher;
pub use xlsm::XlsmImporter;
pub use xlsx::XlsxImporter;
