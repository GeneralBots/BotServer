use super::*;

pub fn get_docs_paper_clarification() -> HashMap<&'static str, &'static str> {
    let mut clarification = HashMap::new();

    clarification.insert(
        "docs_purpose",
        "Docs is a full-featured document editor similar to Google Docs or Microsoft Word. \
         It supports rich text formatting, images, tables, collaboration, comments, \
         version history, and export to multiple formats (PDF, DOCX, HTML)."
    );

    clarification.insert(
        "paper_purpose",
        "Paper is a lightweight notes application similar to Apple Notes or Google Keep. \
         It's designed for quick capture, simple formatting, and fast access to notes. \
         Perfect for meeting notes, quick thoughts, and simple text."
    );

    clarification.insert(
        "when_to_use_docs",
        "Use Docs when you need: rich formatting, collaboration features, \
         document templates, export capabilities, version history, or formal documents."
    );

    clarification.insert(
        "when_to_use_paper",
        "Use Paper when you need: quick note-taking, simple lists, \
         fast capture without formatting overhead, or lightweight text storage."
    );

    clarification.insert(
        "key_differences",
        "Docs = Full document editor (Word-like), Paper = Quick notes (Notes-like). \
         Different tools for different purposes - both available in the Library section."
    );

    clarification
}
pub fn get_library_section_info() -> HashMap<&'static str, &'static str> {
    let mut info = HashMap::new();

    info.insert(
        "name",
        "Library"
    );

    info.insert(
        "description",
        "The Library section contains all content creation and storage apps. \
         This includes Drive (file storage), Docs (full documents), Paper (quick notes), \
         Sheet (spreadsheets), Slides (presentations), Forms (surveys/forms), \
         and Sites (website builder)."
    );

    info.insert(
        "previous_name",
        "Sources"
    );

    info.insert(
        "rename_reason",
        "Renamed from 'Sources' to 'Library' for better clarity. \
         Library better represents a collection of content and documents, \
         while 'Sources' was ambiguous and could be confused with data sources or integrations."
    );

    info
}
