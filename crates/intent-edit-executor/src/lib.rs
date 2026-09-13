use shared_protocol::{
    InstructedDictationTransformRequest, InstructedDictationTransformResponse,
    SelectedTextExecutionAction, SelectedTextExecutionRequest, SelectedTextExecutionResponse,
    SelectedTextProviderDiagnostics, WakePhraseIntentAction, WakePhraseIntentExecutionRequest,
    WakePhraseIntentExecutionResponse,
};

pub trait SelectedTextExecutor: Send + Sync {
    fn execute(
        &self,
        request: &SelectedTextExecutionRequest,
    ) -> Result<SelectedTextExecutionResponse, String>;
}

pub struct ProviderSelectedTextEditRequest<'a> {
    pub session_id: u64,
    pub selected_text: &'a str,
    pub instruction_text: &'a str,
    pub action: &'a SelectedTextExecutionAction,
}

pub trait SelectedTextEditProvider: Send + Sync {
    fn profile_label(&self) -> &str;
    fn model_code(&self) -> &str;
    fn provider_preset(&self) -> Option<String> {
        None
    }
    fn provider_key_source(&self) -> Option<String> {
        None
    }
    fn is_configured(&self) -> bool;
    fn edit(&self, request: &ProviderSelectedTextEditRequest<'_>) -> Result<String, String>;
}

#[derive(Debug)]
pub struct ProviderBackedSelectedTextExecutor<D, P> {
    deterministic: D,
    provider: P,
}

impl<D, P> ProviderBackedSelectedTextExecutor<D, P> {
    pub fn new(deterministic: D, provider: P) -> Self {
        Self {
            deterministic,
            provider,
        }
    }
}

pub trait WakePhraseIntentExecutor: Send + Sync {
    fn execute_intent(
        &self,
        request: &WakePhraseIntentExecutionRequest,
    ) -> Result<WakePhraseIntentExecutionResponse, String>;
}

pub trait InstructedDictationExecutor: Send + Sync {
    fn transform(
        &self,
        request: &InstructedDictationTransformRequest,
    ) -> Result<InstructedDictationTransformResponse, String>;
}

#[derive(Debug, Default)]
pub struct TemporarySelectedTextExecutor;

impl SelectedTextExecutor for TemporarySelectedTextExecutor {
    fn execute(
        &self,
        request: &SelectedTextExecutionRequest,
    ) -> Result<SelectedTextExecutionResponse, String> {
        let normalized_instruction = request.instruction_text.trim().to_ascii_lowercase();
        let instruction_terms = tokenize_instruction(&normalized_instruction);

        if normalized_instruction.is_empty() {
            return Err("selected-text edit instruction was empty".to_string());
        }

        if contains_word(&instruction_terms, "uppercase")
            || contains_phrase(&instruction_terms, &["upper", "case"])
            || contains_phrase(&instruction_terms, &["all", "caps"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::Uppercase,
                request.selected_text.to_uppercase(),
                "temporary executor applied uppercase formatting",
            ));
        }

        if contains_word(&instruction_terms, "lowercase")
            || contains_phrase(&instruction_terms, &["lower", "case"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::Lowercase,
                request.selected_text.to_lowercase(),
                "temporary executor applied lowercase formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["title", "case"]) {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::TitleCase,
                to_title_case(&request.selected_text),
                "temporary executor applied title case formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["sentence", "case"]) {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SentenceCase,
                to_sentence_case(&request.selected_text),
                "temporary executor applied sentence case formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["snake", "case"]) {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SnakeCase,
                to_snake_case(&request.selected_text),
                "temporary executor converted the selection into snake_case",
            ));
        }

        if contains_phrase(&instruction_terms, &["kebab", "case"])
            || contains_phrase(&instruction_terms, &["dash", "case"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::KebabCase,
                to_kebab_case(&request.selected_text),
                "temporary executor converted the selection into kebab-case",
            ));
        }

        if contains_phrase(&instruction_terms, &["camel", "case"]) {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::CamelCase,
                to_camel_case(&request.selected_text),
                "temporary executor converted the selection into camelCase",
            ));
        }

        if contains_phrase(&instruction_terms, &["pascal", "case"]) {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PascalCase,
                to_pascal_case(&request.selected_text),
                "temporary executor converted the selection into PascalCase",
            ));
        }

        if contains_phrase(&instruction_terms, &["constant", "case"])
            || contains_phrase(&instruction_terms, &["screaming", "snake", "case"])
            || contains_phrase(&instruction_terms, &["upper", "snake", "case"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::ConstantCase,
                to_constant_case(&request.selected_text),
                "temporary executor converted the selection into CONSTANT_CASE",
            ));
        }

        if contains_phrase(&instruction_terms, &["inline", "code"])
            || contains_phrase(&instruction_terms, &["wrap", "in", "code"])
            || contains_phrase(&instruction_terms, &["backticks"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::InlineCode,
                to_inline_code(&request.selected_text),
                "temporary executor wrapped the selection in inline code formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["code", "block"])
            || contains_phrase(&instruction_terms, &["wrap", "in", "code", "block"])
            || contains_phrase(&instruction_terms, &["fenced", "code", "block"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::CodeBlock,
                to_code_block(&request.selected_text),
                "temporary executor wrapped the selection in a fenced code block",
            ));
        }

        if contains_phrase(&instruction_terms, &["strip", "code", "fence"])
            || contains_phrase(&instruction_terms, &["remove", "code", "fence"])
            || contains_phrase(&instruction_terms, &["remove", "code", "block", "markers"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::StripCodeFence,
                strip_code_fence(&request.selected_text),
                "temporary executor removed fenced code markers from the selection",
            ));
        }

        if contains_phrase(&instruction_terms, &["markdown", "bold"])
            || contains_phrase(&instruction_terms, &["make", "this", "bold"])
            || contains_phrase(&instruction_terms, &["bold", "this"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::MarkdownBold,
                to_markdown_bold(&request.selected_text),
                "temporary executor wrapped the selection in markdown bold formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["markdown", "italic"])
            || contains_phrase(&instruction_terms, &["make", "this", "italic"])
            || contains_phrase(&instruction_terms, &["italicize", "this"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::MarkdownItalic,
                to_markdown_italic(&request.selected_text),
                "temporary executor wrapped the selection in markdown italic formatting",
            ));
        }

        if contains_phrase(&instruction_terms, &["strip", "markdown", "emphasis"])
            || contains_phrase(&instruction_terms, &["remove", "markdown", "emphasis"])
            || contains_phrase(&instruction_terms, &["remove", "bold", "and", "italic"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::StripMarkdownEmphasis,
                strip_markdown_emphasis(&request.selected_text),
                "temporary executor removed markdown emphasis markers from the selection",
            ));
        }

        if contains_phrase(&instruction_terms, &["wrap", "in", "quotes"])
            || contains_phrase(&instruction_terms, &["put", "this", "in", "quotes"])
            || contains_phrase(&instruction_terms, &["quote", "this"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::WrapInQuotes,
                format!("\"{}\"", request.selected_text),
                "temporary executor wrapped the selection in quotes",
            ));
        }

        if contains_word(&instruction_terms, "bullet")
            || contains_phrase(&instruction_terms, &["bullet", "list"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "list"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::BulletList,
                to_bullet_list(&request.selected_text),
                "temporary executor reformatted the selection as a bullet list",
            ));
        }

        if contains_word(&instruction_terms, "checklist")
            || contains_phrase(&instruction_terms, &["task", "list"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "checklist"])
            || contains_phrase(
                &instruction_terms,
                &["turn", "this", "into", "a", "checklist"],
            )
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::Checklist,
                to_checklist(&request.selected_text),
                "temporary executor reformatted the selection as a checklist",
            ));
        }

        if contains_phrase(&instruction_terms, &["quote", "block"])
            || contains_phrase(&instruction_terms, &["blockquote"])
            || contains_phrase(&instruction_terms, &["format", "as", "quote"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::QuoteBlock,
                to_quote_block(&request.selected_text),
                "temporary executor reformatted the selection as a quote block",
            ));
        }

        if contains_phrase(&instruction_terms, &["numbered", "list"])
            || contains_phrase(
                &instruction_terms,
                &["make", "this", "a", "numbered", "list"],
            )
            || contains_phrase(&instruction_terms, &["number", "these", "lines"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::NumberedList,
                to_numbered_list(&request.selected_text),
                "temporary executor reformatted the selection as a numbered list",
            ));
        }

        if contains_phrase(&instruction_terms, &["sort", "lines"])
            || contains_phrase(&instruction_terms, &["sort", "these", "lines"])
            || contains_phrase(&instruction_terms, &["alphabetize", "these", "lines"])
            || contains_phrase(&instruction_terms, &["alphabetize", "this"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SortLines,
                sort_lines(&request.selected_text),
                "temporary executor sorted the selection line by line",
            ));
        }

        if contains_phrase(&instruction_terms, &["deduplicate", "lines"])
            || contains_phrase(&instruction_terms, &["remove", "duplicate", "lines"])
            || contains_phrase(&instruction_terms, &["remove", "duplicates"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::DeduplicateLines,
                deduplicate_lines(&request.selected_text),
                "temporary executor removed duplicate lines from the selection",
            ));
        }

        if contains_phrase(&instruction_terms, &["remove", "empty", "lines"])
            || contains_phrase(&instruction_terms, &["remove", "blank", "lines"])
            || contains_phrase(&instruction_terms, &["strip", "blank", "lines"])
            || contains_phrase(&instruction_terms, &["strip", "empty", "lines"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::RemoveEmptyLines,
                remove_empty_lines(&request.selected_text),
                "temporary executor removed empty lines from the selection",
            ));
        }

        if contains_phrase(&instruction_terms, &["comma", "separated"])
            || contains_phrase(&instruction_terms, &["comma", "separate", "this"])
            || contains_phrase(&instruction_terms, &["join", "with", "commas"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::CommaSeparated,
                to_comma_separated(&request.selected_text),
                "temporary executor joined the selection into a comma-separated line",
            ));
        }

        if contains_phrase(&instruction_terms, &["pipe", "separated"])
            || contains_phrase(&instruction_terms, &["pipe", "separate", "this"])
            || contains_phrase(&instruction_terms, &["join", "with", "pipes"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PipeSeparated,
                to_pipe_separated(&request.selected_text),
                "temporary executor joined the selection into a pipe-separated line",
            ));
        }

        if contains_phrase(&instruction_terms, &["tab", "separated"])
            || contains_phrase(&instruction_terms, &["tab", "separate", "this"])
            || contains_phrase(&instruction_terms, &["join", "with", "tabs"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::TabSeparated,
                to_tab_separated(&request.selected_text),
                "temporary executor joined the selection into a tab-separated line",
            ));
        }

        if contains_phrase(&instruction_terms, &["semicolon", "separated"])
            || contains_phrase(&instruction_terms, &["semicolon", "separate", "this"])
            || contains_phrase(&instruction_terms, &["join", "with", "semicolons"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SemicolonSeparated,
                to_semicolon_separated(&request.selected_text),
                "temporary executor joined the selection into a semicolon-separated line",
            ));
        }

        if contains_phrase(&instruction_terms, &["json", "array"])
            || contains_phrase(&instruction_terms, &["convert", "to", "json"])
            || contains_phrase(&instruction_terms, &["make", "this", "json"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::JsonArray,
                to_json_array(&request.selected_text),
                "temporary executor converted the selection into a JSON string array",
            ));
        }

        if contains_phrase(&instruction_terms, &["quoted", "csv"])
            || contains_phrase(&instruction_terms, &["csv", "line"])
            || contains_phrase(&instruction_terms, &["make", "this", "csv"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::QuotedCsv,
                to_quoted_csv(&request.selected_text),
                "temporary executor converted the selection into a quoted CSV row",
            ));
        }

        if contains_phrase(&instruction_terms, &["sql", "in", "list"])
            || contains_phrase(&instruction_terms, &["sql", "in", "clause"])
            || contains_phrase(&instruction_terms, &["make", "this", "an", "in", "clause"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SqlInList,
                to_sql_in_list(&request.selected_text),
                "temporary executor converted the selection into a SQL IN-list literal",
            ));
        }

        if contains_phrase(&instruction_terms, &["yaml", "list"])
            || contains_phrase(&instruction_terms, &["make", "this", "yaml"])
            || contains_phrase(&instruction_terms, &["convert", "to", "yaml", "list"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::YamlList,
                to_yaml_list(&request.selected_text),
                "temporary executor converted the selection into a YAML list",
            ));
        }

        if contains_phrase(&instruction_terms, &["yaml", "mapping"])
            || contains_phrase(&instruction_terms, &["yaml", "map"])
            || contains_phrase(
                &instruction_terms,
                &["make", "this", "a", "yaml", "mapping"],
            )
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::YamlMapping,
                to_yaml_mapping(&request.selected_text),
                "temporary executor converted the selection into a YAML mapping",
            ));
        }

        if contains_phrase(&instruction_terms, &["markdown", "table"])
            || contains_phrase(&instruction_terms, &["table", "this"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "table"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::MarkdownTable,
                to_markdown_table(&request.selected_text),
                "temporary executor converted the selection into a one-column markdown table",
            ));
        }

        if contains_phrase(&instruction_terms, &["header", "block"])
            || contains_phrase(&instruction_terms, &["http", "headers"])
            || contains_phrase(&instruction_terms, &["make", "this", "headers"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::HeaderBlock,
                to_header_block(&request.selected_text),
                "temporary executor converted the selection into a header-style block",
            ));
        }

        if contains_phrase(&instruction_terms, &["json", "object"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "json", "object"])
            || contains_phrase(&instruction_terms, &["convert", "to", "json", "object"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::JsonObject,
                to_json_object(&request.selected_text),
                "temporary executor converted the selection into a JSON object",
            ));
        }

        if contains_phrase(&instruction_terms, &["env", "block"])
            || contains_phrase(&instruction_terms, &["dotenv"])
            || contains_phrase(&instruction_terms, &["make", "this", "env"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::EnvBlock,
                to_env_block(&request.selected_text),
                "temporary executor converted the selection into a dotenv-style env block",
            ));
        }

        if contains_phrase(&instruction_terms, &["query", "string"])
            || contains_phrase(&instruction_terms, &["url", "query"])
            || contains_phrase(
                &instruction_terms,
                &["make", "this", "a", "query", "string"],
            )
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::QueryString,
                to_query_string(&request.selected_text),
                "temporary executor converted the selection into a URL query string",
            ));
        }

        if contains_phrase(&instruction_terms, &["toml", "table"])
            || contains_phrase(&instruction_terms, &["make", "this", "toml"])
            || contains_phrase(&instruction_terms, &["convert", "to", "toml", "table"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::TomlTable,
                to_toml_table(&request.selected_text),
                "temporary executor converted the selection into a TOML table block",
            ));
        }

        if contains_phrase(&instruction_terms, &["shell", "exports"])
            || contains_phrase(&instruction_terms, &["bash", "exports"])
            || contains_phrase(&instruction_terms, &["make", "this", "shell", "exports"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::ShellExports,
                to_shell_exports(&request.selected_text),
                "temporary executor converted the selection into POSIX shell export lines",
            ));
        }

        if contains_phrase(&instruction_terms, &["powershell", "env"])
            || contains_phrase(&instruction_terms, &["powershell", "environment"])
            || contains_phrase(&instruction_terms, &["make", "this", "powershell", "env"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PowershellEnv,
                to_powershell_env(&request.selected_text),
                "temporary executor converted the selection into PowerShell env assignments",
            ));
        }

        if contains_phrase(&instruction_terms, &["curl", "headers"])
            || contains_phrase(&instruction_terms, &["curl", "header", "flags"])
            || contains_phrase(&instruction_terms, &["make", "this", "curl", "headers"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::CurlHeaders,
                to_curl_headers(&request.selected_text),
                "temporary executor converted the selection into curl header flags",
            ));
        }

        if contains_phrase(&instruction_terms, &["python", "dict"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "python", "dict"])
            || contains_phrase(&instruction_terms, &["convert", "to", "python", "dict"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PythonDict,
                to_python_dict(&request.selected_text),
                "temporary executor converted the selection into a Python dict literal",
            ));
        }

        if contains_phrase(&instruction_terms, &["javascript", "object"])
            || contains_phrase(&instruction_terms, &["js", "object"])
            || contains_phrase(
                &instruction_terms,
                &["make", "this", "a", "javascript", "object"],
            )
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::JavascriptObject,
                to_javascript_object(&request.selected_text),
                "temporary executor converted the selection into a JavaScript object literal",
            ));
        }

        if contains_phrase(&instruction_terms, &["ruby", "hash"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "ruby", "hash"])
            || contains_phrase(&instruction_terms, &["convert", "to", "ruby", "hash"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::RubyHash,
                to_ruby_hash(&request.selected_text),
                "temporary executor converted the selection into a Ruby hash literal",
            ));
        }

        if contains_phrase(&instruction_terms, &["sql", "values", "rows"])
            || contains_phrase(&instruction_terms, &["values", "rows"])
            || contains_phrase(&instruction_terms, &["make", "this", "sql", "values"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SqlValuesRows,
                to_sql_values_rows(&request.selected_text),
                "temporary executor converted the selection into SQL VALUES rows",
            ));
        }

        if contains_phrase(&instruction_terms, &["strip", "list", "markers"])
            || contains_phrase(&instruction_terms, &["remove", "bullets"])
            || contains_phrase(&instruction_terms, &["remove", "list", "markers"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::StripListMarkers,
                strip_list_markers(&request.selected_text),
                "temporary executor removed bullet and checklist markers from the selection",
            ));
        }

        if contains_phrase(&instruction_terms, &["sentence", "per", "line"])
            || contains_phrase(&instruction_terms, &["one", "sentence", "per", "line"])
            || contains_phrase(&instruction_terms, &["split", "sentences", "into", "lines"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SentencePerLine,
                to_sentence_per_line(&request.selected_text),
                "temporary executor split the selection into one sentence per line",
            ));
        }

        if contains_phrase(&instruction_terms, &["markdown", "heading"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "heading"])
            || contains_phrase(&instruction_terms, &["heading", "level", "two"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::MarkdownHeading,
                to_markdown_heading(&request.selected_text),
                "temporary executor reformatted the selection as a markdown heading",
            ));
        }

        if contains_phrase(&instruction_terms, &["single", "paragraph"])
            || contains_phrase(&instruction_terms, &["one", "paragraph"])
            || contains_phrase(&instruction_terms, &["remove", "line", "breaks"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "paragraph"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::SingleParagraph,
                to_single_paragraph(&request.selected_text),
                "temporary executor collapsed the selection into a single paragraph",
            ));
        }

        if contains_phrase(&instruction_terms, &["clean", "up", "spacing"])
            || contains_phrase(&instruction_terms, &["fix", "spacing"])
            || contains_phrase(&instruction_terms, &["trim", "whitespace"])
            || contains_phrase(&instruction_terms, &["normalize", "spacing"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::CleanupSpacing,
                cleanup_spacing(&request.selected_text),
                "temporary executor normalized spacing across the selection",
            ));
        }

        if contains_word(&instruction_terms, "rewrite")
            || contains_word(&instruction_terms, "reword")
            || contains_phrase(&instruction_terms, &["clean", "this", "up"])
            || contains_phrase(&instruction_terms, &["clean", "this", "text", "up"])
            || contains_phrase(&instruction_terms, &["fix", "grammar"])
            || contains_phrase(&instruction_terms, &["fix", "the", "grammar"])
            || contains_phrase(&instruction_terms, &["fix", "punctuation"])
            || contains_phrase(&instruction_terms, &["make", "this", "clearer"])
            || contains_phrase(&instruction_terms, &["polish", "this"])
            || contains_phrase(&instruction_terms, &["tidy", "this", "up"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PolishWriting,
                polish_writing(&request.selected_text),
                "edit executor polished the selection into cleaner prose",
            ));
        }

        if contains_word(&instruction_terms, "shorten")
            || contains_word(&instruction_terms, "condense")
            || contains_phrase(&instruction_terms, &["make", "this", "shorter"])
            || contains_phrase(&instruction_terms, &["make", "this", "concise"])
            || contains_phrase(&instruction_terms, &["make", "this", "more", "concise"])
            || contains_phrase(&instruction_terms, &["tighten", "this", "up"])
            || contains_phrase(&instruction_terms, &["make", "this", "tighter"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::ConciseRewrite,
                concise_rewrite(&request.selected_text),
                "edit executor tightened the selection into a concise rewrite",
            ));
        }

        if contains_phrase(&instruction_terms, &["make", "this", "professional"])
            || contains_phrase(
                &instruction_terms,
                &["make", "this", "more", "professional"],
            )
            || contains_phrase(&instruction_terms, &["make", "this", "formal"])
            || contains_phrase(&instruction_terms, &["make", "this", "more", "formal"])
            || contains_phrase(&instruction_terms, &["rewrite", "professionally"])
            || contains_phrase(&instruction_terms, &["rewrite", "formally"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::FormalRewrite,
                formal_rewrite(&request.selected_text),
                "edit executor rewrote the selection in a more formal tone",
            ));
        }

        if contains_word(&instruction_terms, "summarize")
            || contains_word(&instruction_terms, "summary")
            || contains_phrase(&instruction_terms, &["bullet", "summary"])
            || contains_phrase(&instruction_terms, &["key", "points"])
            || contains_phrase(&instruction_terms, &["tl", "dr"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::BulletSummary,
                to_bullet_summary(&request.selected_text),
                "edit executor condensed the selection into a bullet summary",
            ));
        }

        if contains_phrase(&instruction_terms, &["turn", "this", "into", "a", "prompt"])
            || contains_phrase(&instruction_terms, &["make", "this", "a", "prompt"])
        {
            return Ok(build_response(
                request,
                &normalized_instruction,
                SelectedTextExecutionAction::PromptScaffold,
                format!(
                    "Write a polished prompt based on the following text:\n\n{}",
                    request.selected_text.trim()
                ),
                "temporary executor expanded the selection into a prompt scaffold",
            ));
        }

        Err(format!(
            "selected-text edit executor does not support instruction `{}` yet",
            request.instruction_text.trim()
        ))
    }
}

impl<D, P> SelectedTextExecutor for ProviderBackedSelectedTextExecutor<D, P>
where
    D: SelectedTextExecutor,
    P: SelectedTextEditProvider,
{
    fn execute(
        &self,
        request: &SelectedTextExecutionRequest,
    ) -> Result<SelectedTextExecutionResponse, String> {
        let deterministic_result = self.deterministic.execute(request);
        let mut deterministic = match deterministic_result {
            Ok(response) => response,
            Err(error) => {
                if request.instruction_text.trim().is_empty() {
                    return Err(error);
                }
                return self.execute_general_provider_edit(request, error);
            }
        };
        if !is_provider_eligible_action(&deterministic.action) {
            return Ok(deterministic);
        }

        let selected_text_char_count = request.selected_text.chars().count();
        let provider_configured = self.provider.is_configured();
        let provider_preset = self.provider.provider_preset();
        let provider_key_source = self.provider.provider_key_source();
        if !provider_configured {
            let output_char_count = deterministic.output_text.chars().count();
            deterministic.strategy = format!(
                "{}; provider fallback: config missing",
                deterministic.strategy
            );
            deterministic.provider_diagnostics = Some(SelectedTextProviderDiagnostics {
                profile_label: self.provider.profile_label().to_string(),
                model_code: self.provider.model_code().to_string(),
                provider_preset: provider_preset.clone(),
                provider_key_source: provider_key_source.clone(),
                provider_attempted: false,
                provider_succeeded: false,
                deterministic_fallback_used: true,
                fallback_reason: Some("provider config missing".to_string()),
                selected_text_char_count,
                output_char_count,
            });
            return Ok(deterministic);
        }

        let provider_request = ProviderSelectedTextEditRequest {
            session_id: request.session_id,
            selected_text: &request.selected_text,
            instruction_text: &request.instruction_text,
            action: &deterministic.action,
        };

        match self.provider.edit(&provider_request) {
            Ok(output) => {
                let trimmed = output.trim();
                if is_unusable_provider_edit_output(trimmed) {
                    let output_char_count = deterministic.output_text.chars().count();
                    deterministic.strategy = format!(
                        "{}; provider fallback: unusable output",
                        deterministic.strategy
                    );
                    deterministic.provider_diagnostics = Some(SelectedTextProviderDiagnostics {
                        profile_label: self.provider.profile_label().to_string(),
                        model_code: self.provider.model_code().to_string(),
                        provider_preset: self.provider.provider_preset(),
                        provider_key_source: self.provider.provider_key_source(),
                        provider_attempted: true,
                        provider_succeeded: false,
                        deterministic_fallback_used: true,
                        fallback_reason: Some("provider output was empty or unusable".to_string()),
                        selected_text_char_count,
                        output_char_count,
                    });
                    Ok(deterministic)
                } else {
                    let output_text = trimmed.to_string();
                    let output_char_count = output_text.chars().count();
                    Ok(SelectedTextExecutionResponse {
                        output_text,
                        action: deterministic.action,
                        strategy: "provider edited the selected text".to_string(),
                        normalized_instruction: deterministic.normalized_instruction,
                        provider_diagnostics: Some(SelectedTextProviderDiagnostics {
                            profile_label: self.provider.profile_label().to_string(),
                            model_code: self.provider.model_code().to_string(),
                            provider_preset: self.provider.provider_preset(),
                            provider_key_source: self.provider.provider_key_source(),
                            provider_attempted: true,
                            provider_succeeded: true,
                            deterministic_fallback_used: false,
                            fallback_reason: None,
                            selected_text_char_count,
                            output_char_count,
                        }),
                    })
                }
            }
            Err(error) => {
                let output_char_count = deterministic.output_text.chars().count();
                deterministic.strategy = format!(
                    "{}; provider fallback: request failed",
                    deterministic.strategy
                );
                deterministic.provider_diagnostics = Some(SelectedTextProviderDiagnostics {
                    profile_label: self.provider.profile_label().to_string(),
                    model_code: self.provider.model_code().to_string(),
                    provider_preset: self.provider.provider_preset(),
                    provider_key_source: self.provider.provider_key_source(),
                    provider_attempted: true,
                    provider_succeeded: false,
                    deterministic_fallback_used: true,
                    fallback_reason: Some(error),
                    selected_text_char_count,
                    output_char_count,
                });
                Ok(deterministic)
            }
        }
    }
}

impl<D, P> ProviderBackedSelectedTextExecutor<D, P>
where
    P: SelectedTextEditProvider,
{
    fn execute_general_provider_edit(
        &self,
        request: &SelectedTextExecutionRequest,
        deterministic_error: String,
    ) -> Result<SelectedTextExecutionResponse, String> {
        let action = SelectedTextExecutionAction::GeneralProviderEdit;
        let selected_text_char_count = request.selected_text.chars().count();
        let normalized_instruction = request.instruction_text.trim().to_string();

        if !self.provider.is_configured() {
            return Err(format!(
                "selected-text provider config missing; freeform edit requires provider ({deterministic_error})"
            ));
        }

        let provider_request = ProviderSelectedTextEditRequest {
            session_id: request.session_id,
            selected_text: &request.selected_text,
            instruction_text: &request.instruction_text,
            action: &action,
        };

        match self.provider.edit(&provider_request) {
            Ok(output) => {
                let trimmed = output.trim();
                if is_unusable_provider_edit_output(trimmed) {
                    return Err(
                        "selected-text provider returned empty or unusable output for freeform edit"
                            .to_string(),
                    );
                }

                let output_text = trimmed.to_string();
                let output_char_count = output_text.chars().count();
                Ok(SelectedTextExecutionResponse {
                    output_text,
                    action,
                    strategy: "provider handled a freeform selected-text edit".to_string(),
                    normalized_instruction,
                    provider_diagnostics: Some(SelectedTextProviderDiagnostics {
                        profile_label: self.provider.profile_label().to_string(),
                        model_code: self.provider.model_code().to_string(),
                        provider_preset: self.provider.provider_preset(),
                        provider_key_source: self.provider.provider_key_source(),
                        provider_attempted: true,
                        provider_succeeded: true,
                        deterministic_fallback_used: false,
                        fallback_reason: None,
                        selected_text_char_count,
                        output_char_count,
                    }),
                })
            }
            Err(error) => Err(format!(
                "selected-text provider freeform edit failed: {error}"
            )),
        }
    }
}

impl WakePhraseIntentExecutor for TemporarySelectedTextExecutor {
    fn execute_intent(
        &self,
        request: &WakePhraseIntentExecutionRequest,
    ) -> Result<WakePhraseIntentExecutionResponse, String> {
        let normalized_command = request.command_text.trim().to_ascii_lowercase();
        let command_terms = tokenize_instruction(&normalized_command);

        if normalized_command.is_empty() {
            return Err("wake-phrase intent command was empty".to_string());
        }

        if contains_word(&command_terms, "email")
            || contains_phrase(&command_terms, &["draft", "an", "email"])
            || contains_phrase(&command_terms, &["write", "an", "email"])
            || contains_phrase(&command_terms, &["help", "me", "write", "an", "email"])
            || contains_phrase(&command_terms, &["help", "me", "draft", "an", "email"])
        {
            let details = parse_email_request_details(request.command_text.trim());
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "Subject: {}\n\nHi,\n\n{}\n\nBest,\n[Your Name]",
                    build_email_subject_line(&details),
                    build_email_body(&details)
                ),
                action: WakePhraseIntentAction::DraftEmail,
                strategy: "temporary intent executor expanded the request into an email draft"
                    .to_string(),
                normalized_command,
            });
        }

        if contains_phrase(&command_terms, &["reply", "to"])
            || contains_phrase(&command_terms, &["respond", "to"])
            || contains_phrase(&command_terms, &["write", "a", "reply"])
        {
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "Hi,\n\n{}\n\nBest,\n[Your Name]",
                    build_reply_body(request.command_text.trim())
                ),
                action: WakePhraseIntentAction::ReplyMessage,
                strategy: "temporary intent executor expanded the request into a reply draft"
                    .to_string(),
                normalized_command,
            });
        }

        if contains_word(&command_terms, "summarize")
            || contains_phrase(&command_terms, &["give", "me", "a", "summary"])
        {
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "Summary:\n- {}\n- Key next step: refine the requested outcome.\n- Open question: add any missing constraints or audience details.",
                    sentence_case_fragment(request.command_text.trim())
                ),
                action: WakePhraseIntentAction::Summarize,
                strategy: "temporary intent executor converted the request into a summary scaffold"
                    .to_string(),
                normalized_command,
            });
        }

        if contains_word(&command_terms, "checklist")
            || contains_phrase(&command_terms, &["to", "do", "list"])
            || contains_phrase(&command_terms, &["todo", "list"])
            || contains_phrase(&command_terms, &["make", "a", "checklist"])
        {
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "- Clarify the requested outcome: {}\n- Gather the needed inputs or constraints.\n- Complete the core task.\n- Review and send the final result.",
                    strip_checklist_prefix(request.command_text.trim())
                ),
                action: WakePhraseIntentAction::Checklist,
                strategy: "temporary intent executor expanded the request into a checklist"
                    .to_string(),
                normalized_command,
            });
        }

        if contains_word(&command_terms, "plan")
            || contains_phrase(&command_terms, &["bullet", "plan"])
            || contains_phrase(&command_terms, &["make", "a", "plan"])
        {
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "- Clarify the goal: {}\n- Break the work into milestones.\n- Execute the highest-leverage first step.\n- Review the result and iterate.",
                    request.command_text.trim()
                ),
                action: WakePhraseIntentAction::BulletPlan,
                strategy: "temporary intent executor expanded the request into a bullet plan"
                    .to_string(),
                normalized_command,
            });
        }

        if contains_word(&command_terms, "rewrite")
            || contains_phrase(&command_terms, &["make", "this", "clearer"])
            || contains_phrase(&command_terms, &["make", "this", "more", "polite"])
        {
            return Ok(WakePhraseIntentExecutionResponse {
                output_text: format!(
                    "Here is a cleaner version:\n\n{}",
                    sentence_case_fragment(request.command_text.trim())
                ),
                action: WakePhraseIntentAction::Rewrite,
                strategy: "temporary intent executor generated a rewrite scaffold".to_string(),
                normalized_command,
            });
        }

        Ok(WakePhraseIntentExecutionResponse {
            output_text: format!(
                "Draft:\n{}\n\nNext step: add any missing audience, tone, or format constraints.",
                sentence_case_fragment(request.command_text.trim())
            ),
            action: WakePhraseIntentAction::GeneralDraft,
            strategy: "temporary intent executor produced a general drafting scaffold".to_string(),
            normalized_command,
        })
    }
}

impl InstructedDictationExecutor for TemporarySelectedTextExecutor {
    fn transform(
        &self,
        _request: &InstructedDictationTransformRequest,
    ) -> Result<InstructedDictationTransformResponse, String> {
        Err("instructed dictation requires a configured provider".to_string())
    }
}

fn build_response(
    _request: &SelectedTextExecutionRequest,
    normalized_instruction: &str,
    action: SelectedTextExecutionAction,
    output_text: String,
    strategy: &str,
) -> SelectedTextExecutionResponse {
    SelectedTextExecutionResponse {
        output_text,
        action,
        strategy: strategy.to_string(),
        normalized_instruction: normalized_instruction.to_string(),
        provider_diagnostics: None,
    }
}

fn is_provider_eligible_action(action: &SelectedTextExecutionAction) -> bool {
    matches!(
        action,
        SelectedTextExecutionAction::PolishWriting
            | SelectedTextExecutionAction::ConciseRewrite
            | SelectedTextExecutionAction::FormalRewrite
            | SelectedTextExecutionAction::BulletSummary
            | SelectedTextExecutionAction::GeneralProviderEdit
    )
}

fn is_unusable_provider_edit_output(output: &str) -> bool {
    output.trim().is_empty()
        || (output.contains("Instruction:") && output.contains("Selected text:"))
        || output.eq_ignore_ascii_case("null")
}

fn tokenize_instruction(input: &str) -> Vec<&str> {
    input
        .split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|term| !term.is_empty())
        .collect()
}

fn contains_word(terms: &[&str], target: &str) -> bool {
    terms.iter().any(|term| *term == target)
}

fn contains_phrase(terms: &[&str], phrase: &[&str]) -> bool {
    if phrase.is_empty() || phrase.len() > terms.len() {
        return false;
    }

    terms.windows(phrase.len()).any(|window| window == phrase)
}

fn to_title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };

            let mut titled = first.to_uppercase().collect::<String>();
            titled.push_str(&chars.as_str().to_lowercase());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_sentence_case(input: &str) -> String {
    let trimmed = input.trim();
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    let mut output = first.to_uppercase().collect::<String>();
    output.push_str(&chars.as_str().to_lowercase());
    output
}

fn identifier_words(input: &str) -> Vec<String> {
    input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_lowercase())
        .collect()
}

fn to_snake_case(input: &str) -> String {
    identifier_words(input).join("_")
}

fn to_kebab_case(input: &str) -> String {
    identifier_words(input).join("-")
}

fn to_camel_case(input: &str) -> String {
    let mut words = identifier_words(input).into_iter();
    let Some(first) = words.next() else {
        return String::new();
    };

    let mut output = first;
    for word in words {
        let mut chars = word.chars();
        let Some(first_char) = chars.next() else {
            continue;
        };
        output.push_str(&first_char.to_uppercase().collect::<String>());
        output.push_str(chars.as_str());
    }
    output
}

fn to_pascal_case(input: &str) -> String {
    identifier_words(input)
        .into_iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first_char) = chars.next() else {
                return String::new();
            };
            let mut output = first_char.to_uppercase().collect::<String>();
            output.push_str(chars.as_str());
            output
        })
        .collect::<String>()
}

fn to_constant_case(input: &str) -> String {
    identifier_words(input)
        .into_iter()
        .map(|word| word.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

fn to_inline_code(input: &str) -> String {
    format!("`{}`", input.trim())
}

fn to_code_block(input: &str) -> String {
    format!("```\n{}\n```", input.trim())
}

fn strip_code_fence(input: &str) -> String {
    let trimmed = input.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let rest = if let Some(index) = rest.find('\n') {
        let first_line = &rest[..index];
        if !first_line.trim().is_empty() && !first_line.contains(' ') {
            &rest[index + 1..]
        } else {
            rest
        }
    } else {
        rest
    };
    rest.trim_end_matches('`').trim().to_string()
}

fn to_markdown_bold(input: &str) -> String {
    format!("**{}**", input.trim())
}

fn to_markdown_italic(input: &str) -> String {
    format!("*{}*", input.trim())
}

fn strip_markdown_emphasis(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .map(strip_markdown_emphasis_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_markdown_emphasis_line(line: &str) -> String {
    let mut value = line.trim();
    loop {
        let next = value
            .strip_prefix("**")
            .and_then(|inner| inner.strip_suffix("**"))
            .or_else(|| {
                value
                    .strip_prefix("__")
                    .and_then(|inner| inner.strip_suffix("__"))
            })
            .or_else(|| {
                value
                    .strip_prefix('*')
                    .and_then(|inner| inner.strip_suffix('*'))
            })
            .or_else(|| {
                value
                    .strip_prefix('_')
                    .and_then(|inner| inner.strip_suffix('_'))
            });
        let Some(inner) = next else {
            break;
        };
        value = inner.trim();
    }
    value.to_string()
}

fn sentence_case_fragment(input: &str) -> String {
    let mut output = to_sentence_case(input);
    if !output.ends_with(['.', '!', '?', '。', '！', '？']) {
        output.push('.');
    }
    output
}

fn to_bullet_list(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_checklist(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| format!("- [ ] {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_quote_block(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_numbered_list(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(index, line)| format!("{}. {line}", index + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sort_lines(input: &str) -> String {
    let mut lines = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    lines.sort_by_cached_key(|line| line.to_ascii_lowercase());
    lines.join("\n")
}

fn deduplicate_lines(input: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let normalized = line.to_ascii_lowercase();
            if seen.insert(normalized) {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_empty_lines(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_comma_separated(input: &str) -> String {
    list_segments(input).join(", ")
}

fn to_pipe_separated(input: &str) -> String {
    list_segments(input).join(" | ")
}

fn to_tab_separated(input: &str) -> String {
    list_segments(input).join("\t")
}

fn to_semicolon_separated(input: &str) -> String {
    list_segments(input).join("; ")
}

fn to_json_array(input: &str) -> String {
    let items = list_segments(input)
        .into_iter()
        .map(|segment| format!("  \"{}\"", json_escape(&segment)))
        .collect::<Vec<_>>();

    if items.is_empty() {
        "[]".to_string()
    } else {
        format!("[\n{}\n]", items.join(",\n"))
    }
}

fn to_quoted_csv(input: &str) -> String {
    list_segments(input)
        .into_iter()
        .map(|segment| format!("\"{}\"", csv_escape(&segment)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn to_sql_in_list(input: &str) -> String {
    let items = list_segments(input)
        .into_iter()
        .map(|segment| format!("'{}'", sql_escape(&segment)))
        .collect::<Vec<_>>();

    format!("({})", items.join(", "))
}

fn to_yaml_list(input: &str) -> String {
    list_segments(input)
        .into_iter()
        .map(|segment| format!("- {}", yaml_escape(&segment)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KeyValueSegment {
    key: String,
    value: String,
    value_was_quoted: bool,
}

fn to_yaml_mapping(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "{}: {}",
                yaml_key_escape(&segment.key),
                yaml_escape(&segment.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_markdown_table(input: &str) -> String {
    let rows = list_segments(input)
        .into_iter()
        .map(|segment| format!("| {} |", markdown_table_escape(&segment)))
        .collect::<Vec<_>>();

    if rows.is_empty() {
        "| Value |\n| --- |".to_string()
    } else {
        format!("| Value |\n| --- |\n{}", rows.join("\n"))
    }
}

fn to_header_block(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            if segment.value.is_empty() {
                format!("{}:", segment.key)
            } else {
                format!("{}: {}", segment.key, segment.value)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_json_object(input: &str) -> String {
    let items = key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "  \"{}\": {}",
                json_escape(&segment.key),
                render_json_value(&segment.value, segment.value_was_quoted)
            )
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{}\n}}", items.join(",\n"))
    }
}

fn to_env_block(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "{}={}",
                env_key_normalize(&segment.key),
                env_value_escape(&segment.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_query_string(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "{}={}",
                url_encode(&segment.key),
                url_encode(&segment.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn to_toml_table(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "{} = {}",
                toml_key_normalize(&segment.key),
                render_toml_value(&segment.value, segment.value_was_quoted)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_shell_exports(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "export {}={}",
                env_key_normalize(&segment.key),
                shell_single_quote(&segment.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_powershell_env(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "$env:{} = \"{}\"",
                env_key_normalize(&segment.key),
                powershell_escape(&segment.value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_curl_headers(input: &str) -> String {
    key_value_segments(input)
        .into_iter()
        .map(|segment| {
            if segment.value.is_empty() {
                format!("-H {}", shell_single_quote(&format!("{}:", segment.key)))
            } else {
                format!(
                    "-H {}",
                    shell_single_quote(&format!("{}: {}", segment.key, segment.value))
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" \\\n")
}

fn to_python_dict(input: &str) -> String {
    let items = key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "    '{}': {}",
                py_escape(&segment.key),
                render_python_value(&segment.value, segment.value_was_quoted)
            )
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{}\n}}", items.join(",\n"))
    }
}

fn to_javascript_object(input: &str) -> String {
    let items = key_value_segments(input)
        .into_iter()
        .map(|segment| {
            let rendered_key = if is_js_identifier(&segment.key) {
                segment.key
            } else {
                format!("\"{}\"", json_escape(&segment.key))
            };
            format!(
                "  {}: {}",
                rendered_key,
                render_javascript_value(&segment.value, segment.value_was_quoted)
            )
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{}\n}}", items.join(",\n"))
    }
}

fn to_ruby_hash(input: &str) -> String {
    let items = key_value_segments(input)
        .into_iter()
        .map(|segment| {
            format!(
                "  '{}' => {}",
                ruby_escape(&segment.key),
                render_ruby_value(&segment.value, segment.value_was_quoted)
            )
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{}\n}}", items.join(",\n"))
    }
}

fn to_sql_values_rows(input: &str) -> String {
    list_segments(input)
        .into_iter()
        .map(|segment| format!("('{}')", sql_escape(&segment)))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn list_segments(input: &str) -> Vec<String> {
    input
        .lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn key_value_segments(input: &str) -> Vec<KeyValueSegment> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .flat_map(split_key_value_candidates)
        .enumerate()
        .filter_map(|(index, line)| parse_key_value_line(&line, index))
        .collect()
}

fn split_key_value_candidates(line: &str) -> Vec<String> {
    let trimmed = line.trim();

    if let Some(candidates) = extract_direct_candidates(trimmed) {
        return candidates;
    }

    for candidate_source in candidate_split_sources(trimmed) {
        if candidate_source.contains('&') && candidate_source.contains('=') {
            let candidates = split_top_level(&candidate_source, '&');
            if candidates.len() > 1
                && candidates
                    .iter()
                    .all(|item| looks_like_key_value_candidate(item))
            {
                return candidates;
            }
        }

        if candidate_source.contains(',')
            && (candidate_source.contains(':') || candidate_source.contains('='))
        {
            let candidates = split_top_level(&candidate_source, ',');
            if candidates.len() > 1
                && candidates
                    .iter()
                    .all(|item| looks_like_key_value_candidate(item))
            {
                return candidates;
            }
        }

        if candidate_source.contains(';')
            && (candidate_source.contains(':') || candidate_source.contains('='))
        {
            let candidates = split_top_level(&candidate_source, ';');
            if candidates.len() > 1
                && candidates
                    .iter()
                    .all(|item| looks_like_key_value_candidate(item))
            {
                return candidates;
            }
        }

        if candidate_source != trimmed && looks_like_key_value_candidate(&candidate_source) {
            return vec![candidate_source];
        }
    }

    vec![trimmed.to_string()]
}

fn candidate_split_sources(line: &str) -> Vec<String> {
    let mut sources = Vec::new();

    if let Some(query) = extract_url_query_source(line) {
        sources.push(query);
    }

    if let Some(unwrapped) = unwrap_balanced_outer_object(line) {
        sources.push(unwrapped.to_string());
    }

    for payload in extract_curl_data_sources(line) {
        if let Some(unwrapped) = unwrap_balanced_outer_object(&payload) {
            sources.push(unwrapped.to_string());
        }
        sources.push(payload);
    }

    sources.push(line.to_string());

    sources
}

fn extract_direct_candidates(line: &str) -> Option<Vec<String>> {
    let curl_headers = extract_curl_header_candidates(line);
    if curl_headers.len() > 1 {
        return Some(curl_headers);
    }

    None
}

fn unwrap_balanced_outer_object(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    let object_start = if trimmed.starts_with("@{") && trimmed.ends_with('}') {
        2
    } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
        1
    } else {
        return None;
    };

    let mut in_single = false;
    let mut in_double = false;
    let mut brace_depth = 0_u32;

    for (idx, ch) in trimmed.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '{' if !in_single && !in_double => brace_depth += 1,
            '}' if !in_single && !in_double && brace_depth > 0 => {
                brace_depth -= 1;
                if brace_depth == 0 && idx != trimmed.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }

    if brace_depth == 0 && !in_single && !in_double {
        Some(trimmed[object_start..trimmed.len() - 1].trim())
    } else {
        None
    }
}

fn looks_like_key_value_candidate(input: &str) -> bool {
    let trimmed = trim_structural_line_wrappers(input.trim());

    if trimmed.is_empty()
        || trimmed == "{"
        || trimmed == "}"
        || trimmed == "["
        || trimmed == "]"
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || is_comment_only_line(trimmed)
    {
        return false;
    }

    trimmed.starts_with("export ")
        || trimmed.starts_with("$env:")
        || trimmed.starts_with("-H ")
        || trimmed.contains("=>")
        || trimmed.contains('=')
        || trimmed.contains(':')
}

fn extract_url_query_source(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let query_start = trimmed.find('?')?;

    let token_start = trimmed[..query_start]
        .rfind(char::is_whitespace)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let token_end = trimmed[query_start..]
        .find(char::is_whitespace)
        .map(|idx| query_start + idx)
        .unwrap_or(trimmed.len());

    let token = strip_outer_quotes(
        trimmed[token_start..token_end]
            .trim_matches(|ch| matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | ',')),
    );

    let (_, query) = token.split_once('?')?;
    let query = if let Some(fragment_start) = query.find('#') {
        &query[..fragment_start]
    } else {
        query
    };

    let query = query
        .trim()
        .trim_end_matches(|ch| matches!(ch, '"' | '\'' | ')' | ']' | '}' | ','));
    if query.contains('=') {
        Some(query.to_string())
    } else {
        None
    }
}

fn extract_curl_header_candidates(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if !trimmed.contains("-H") && !trimmed.contains("--header") {
        return Vec::new();
    }

    let mut items = Vec::new();
    let mut index = 0;

    while index < trimmed.len() {
        let remainder = &trimmed[index..];
        let Some(relative_start) = find_next_header_flag(remainder) else {
            break;
        };
        index += relative_start;

        let (segment, consumed) = consume_header_flag(&trimmed[index..]);
        if let Some(candidate) = segment {
            items.push(candidate);
        }

        if consumed == 0 {
            break;
        }
        index += consumed;
    }

    items
}

fn extract_curl_data_sources(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if !trimmed.contains("-d ")
        && !trimmed.contains("--data ")
        && !trimmed.contains("--data-raw ")
        && !trimmed.contains("--data-binary ")
        && !trimmed.contains("--data-urlencode ")
    {
        return Vec::new();
    }

    let mut items = Vec::new();
    let mut index = 0;

    while index < trimmed.len() {
        let remainder = &trimmed[index..];
        let Some(relative_start) = find_next_data_flag(remainder) else {
            break;
        };
        index += relative_start;

        let (segment, consumed) = consume_data_flag(&trimmed[index..]);
        if let Some(candidate) = segment {
            items.push(candidate);
        }

        if consumed == 0 {
            break;
        }
        index += consumed;
    }

    items
}

fn find_next_header_flag(input: &str) -> Option<usize> {
    let short = input.find("-H ");
    let long = input.find("--header ");

    match (short, long) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn find_next_data_flag(input: &str) -> Option<usize> {
    let candidates = [
        input.find("-d "),
        input.find("--data "),
        input.find("--data-raw "),
        input.find("--data-binary "),
        input.find("--data-urlencode "),
    ];

    candidates.into_iter().flatten().min()
}

fn consume_header_flag(input: &str) -> (Option<String>, usize) {
    let (prefix, mut index) = if input.starts_with("--header ") {
        ("--header ", "--header ".len())
    } else if input.starts_with("-H ") {
        ("-H ", "-H ".len())
    } else {
        return (None, 0);
    };

    while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }

    if index >= input.len() {
        return (None, prefix.len());
    }

    let bytes = input.as_bytes();
    let start = index;
    let first = bytes[index];

    if first == b'\'' || first == b'"' {
        let quote = first;
        index += 1;
        while index < input.len() {
            if bytes[index] == quote {
                index += 1;
                let segment = input[start..index].trim().to_string();
                return (Some(format!("-H {}", segment)), index);
            }
            index += 1;
        }

        let segment = input[start..].trim().to_string();
        return (Some(format!("-H {}", segment)), input.len());
    }

    while index < input.len() {
        if input[index..].starts_with(" -H ") || input[index..].starts_with(" --header ") {
            break;
        }
        index += 1;
    }

    let segment = input[start..index].trim();
    if segment.is_empty() {
        (None, index)
    } else {
        (Some(format!("-H {}", segment)), index)
    }
}

fn consume_data_flag(input: &str) -> (Option<String>, usize) {
    let mut index = if input.starts_with("--data-urlencode ") {
        "--data-urlencode ".len()
    } else if input.starts_with("--data-binary ") {
        "--data-binary ".len()
    } else if input.starts_with("--data-raw ") {
        "--data-raw ".len()
    } else if input.starts_with("--data ") {
        "--data ".len()
    } else if input.starts_with("-d ") {
        "-d ".len()
    } else {
        return (None, 0);
    };

    while index < input.len() && input.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }

    if index >= input.len() {
        return (None, index);
    }

    let bytes = input.as_bytes();
    let start = index;
    let first = bytes[index];

    let raw_segment = if first == b'\'' || first == b'"' {
        let quote = first;
        index += 1;
        while index < input.len() {
            if bytes[index] == quote {
                index += 1;
                break;
            }
            index += 1;
        }
        input[start..index.min(input.len())].trim().to_string()
    } else {
        while index < input.len() {
            if input[index..].starts_with(" -d ")
                || input[index..].starts_with(" --data ")
                || input[index..].starts_with(" --data-raw ")
                || input[index..].starts_with(" --data-binary ")
                || input[index..].starts_with(" --data-urlencode ")
                || input[index..].starts_with(" -H ")
                || input[index..].starts_with(" --header ")
            {
                break;
            }
            index += 1;
        }
        input[start..index].trim().to_string()
    };

    let payload = strip_outer_quotes(raw_segment.trim())
        .trim_start_matches('@')
        .trim();
    if payload.is_empty() {
        (None, index)
    } else {
        (Some(payload.to_string()), index)
    }
}

fn parse_key_value_line(line: &str, index: usize) -> Option<KeyValueSegment> {
    let trimmed = trim_structural_line_wrappers(line.trim());

    if trimmed.is_empty()
        || trimmed == "{"
        || trimmed == "}"
        || trimmed == "["
        || trimmed == "]"
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || is_comment_only_line(trimmed)
    {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("export ") {
        return Some(normalize_key_value_pair(rest, '=', index));
    }

    if let Some(rest) = trimmed.strip_prefix("$env:") {
        return Some(normalize_key_value_pair(rest, '=', index));
    }

    if let Some(rest) = trimmed.strip_prefix("-H ") {
        let header_text = strip_outer_quotes(rest.trim());
        return Some(normalize_key_value_pair(header_text, ':', index));
    }

    if let Some((key, value)) = trimmed.split_once("=>") {
        return Some(normalize_key_value_parts(key, value, index));
    }

    if let Some((key, value)) = trimmed.split_once('=') {
        return Some(normalize_key_value_parts(key, value, index));
    }

    if let Some((key, value)) = trimmed.split_once(':') {
        return Some(normalize_key_value_parts(key, value, index));
    }

    Some(KeyValueSegment {
        key: strip_outer_quotes(trimmed).to_string(),
        value: String::new(),
        value_was_quoted: false,
    })
}

fn normalize_key_value_pair(input: &str, separator: char, index: usize) -> KeyValueSegment {
    if let Some((key, value)) = input.split_once(separator) {
        normalize_key_value_parts(key, value, index)
    } else {
        KeyValueSegment {
            key: strip_outer_quotes(input.trim()).to_string(),
            value: String::new(),
            value_was_quoted: false,
        }
    }
}

fn normalize_key_value_parts(raw_key: &str, raw_value: &str, index: usize) -> KeyValueSegment {
    let key = strip_outer_quotes(trim_structural_part_wrappers(raw_key.trim()));
    let raw_value = trim_structural_part_wrappers(strip_inline_value_comment(raw_value.trim()));
    let (value, value_was_quoted) = strip_outer_quotes_with_flag(raw_value);

    let normalized_key = if key.is_empty() {
        format!("item{}", index + 1)
    } else {
        key.to_string()
    };

    KeyValueSegment {
        key: normalized_key,
        value: value.to_string(),
        value_was_quoted,
    }
}

fn json_escape(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn csv_escape(input: &str) -> String {
    input.replace('"', "\"\"")
}

fn sql_escape(input: &str) -> String {
    input.replace('\'', "''")
}

fn yaml_escape(input: &str) -> String {
    if input.contains(':')
        || input.contains('#')
        || input.starts_with('-')
        || input.starts_with('[')
        || input.starts_with('{')
    {
        format!("\"{}\"", input.replace('"', "\\\""))
    } else {
        input.to_string()
    }
}

fn yaml_key_escape(input: &str) -> String {
    if input.contains(':')
        || input.contains('#')
        || input.contains(' ')
        || input.starts_with('-')
        || input.starts_with('[')
        || input.starts_with('{')
    {
        format!("\"{}\"", input.replace('"', "\\\""))
    } else {
        input.to_string()
    }
}

fn markdown_table_escape(input: &str) -> String {
    input.replace('|', "\\|")
}

fn env_key_normalize(input: &str) -> String {
    let mut normalized = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_uppercase());
        } else {
            normalized.push('_');
        }
    }
    normalized.trim_matches('_').to_string()
}

fn env_value_escape(input: &str) -> String {
    if input.contains(' ') || input.contains('#') || input.contains('"') {
        format!("\"{}\"", input.replace('"', "\\\""))
    } else {
        input.to_string()
    }
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

fn toml_key_normalize(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        trimmed.to_string()
    } else {
        format!("\"{}\"", trimmed.replace('"', "\\\""))
    }
}

fn toml_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarKind {
    BoolTrue,
    BoolFalse,
    Nullish,
    Number,
}

fn infer_scalar_kind(value: &str, value_was_quoted: bool) -> Option<ScalarKind> {
    if value_was_quoted {
        return None;
    }

    let normalized = value.trim();
    let lowercase = normalized.to_ascii_lowercase();

    match lowercase.as_str() {
        "true" => return Some(ScalarKind::BoolTrue),
        "false" => return Some(ScalarKind::BoolFalse),
        "null" | "none" | "nil" => return Some(ScalarKind::Nullish),
        _ => {}
    }

    if looks_like_number_literal(normalized) {
        Some(ScalarKind::Number)
    } else {
        None
    }
}

fn looks_like_number_literal(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    if value.parse::<i64>().is_ok() {
        let digits = value.strip_prefix('-').unwrap_or(value);
        return digits == "0" || !digits.starts_with('0');
    }

    (value.contains('.') || value.contains('e') || value.contains('E'))
        && value.parse::<f64>().is_ok()
}

fn render_json_value(value: &str, value_was_quoted: bool) -> String {
    match infer_scalar_kind(value, value_was_quoted) {
        Some(ScalarKind::BoolTrue) => "true".to_string(),
        Some(ScalarKind::BoolFalse) => "false".to_string(),
        Some(ScalarKind::Nullish) => "null".to_string(),
        Some(ScalarKind::Number) => value.to_string(),
        None => format!("\"{}\"", json_escape(value)),
    }
}

fn render_javascript_value(value: &str, value_was_quoted: bool) -> String {
    match infer_scalar_kind(value, value_was_quoted) {
        Some(ScalarKind::BoolTrue) => "true".to_string(),
        Some(ScalarKind::BoolFalse) => "false".to_string(),
        Some(ScalarKind::Nullish) => "null".to_string(),
        Some(ScalarKind::Number) => value.to_string(),
        None => format!("\"{}\"", json_escape(value)),
    }
}

fn render_python_value(value: &str, value_was_quoted: bool) -> String {
    match infer_scalar_kind(value, value_was_quoted) {
        Some(ScalarKind::BoolTrue) => "True".to_string(),
        Some(ScalarKind::BoolFalse) => "False".to_string(),
        Some(ScalarKind::Nullish) => "None".to_string(),
        Some(ScalarKind::Number) => value.to_string(),
        None => format!("'{}'", py_escape(value)),
    }
}

fn render_ruby_value(value: &str, value_was_quoted: bool) -> String {
    match infer_scalar_kind(value, value_was_quoted) {
        Some(ScalarKind::BoolTrue) => "true".to_string(),
        Some(ScalarKind::BoolFalse) => "false".to_string(),
        Some(ScalarKind::Nullish) => "nil".to_string(),
        Some(ScalarKind::Number) => value.to_string(),
        None => format!("'{}'", ruby_escape(value)),
    }
}

fn render_toml_value(value: &str, value_was_quoted: bool) -> String {
    match infer_scalar_kind(value, value_was_quoted) {
        Some(ScalarKind::BoolTrue) => "true".to_string(),
        Some(ScalarKind::BoolFalse) => "false".to_string(),
        Some(ScalarKind::Number) => value.to_string(),
        Some(ScalarKind::Nullish) | None => format!("\"{}\"", toml_escape(value)),
    }
}

fn strip_outer_quotes(input: &str) -> &str {
    strip_outer_quotes_with_flag(input).0
}

fn strip_outer_quotes_with_flag(input: &str) -> (&str, bool) {
    if input.len() >= 2 {
        let bytes = input.as_bytes();
        let first = bytes[0];
        let last = bytes[input.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return (&input[1..input.len() - 1], true);
        }
    }

    (input, false)
}

fn trim_structural_line_wrappers(input: &str) -> &str {
    input.trim_matches(|ch| matches!(ch, ',' | '{' | '}'))
}

fn trim_structural_part_wrappers(input: &str) -> &str {
    input.trim_matches(|ch| matches!(ch, ',' | '{' | '}'))
}

fn is_comment_only_line(input: &str) -> bool {
    input.starts_with('#') || input.starts_with(';') || input.starts_with("//")
}

fn strip_inline_value_comment(input: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_whitespace = true;
    let mut iter = input.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                prev_was_whitespace = false;
            }
            '"' if !in_single => {
                in_double = !in_double;
                prev_was_whitespace = false;
            }
            '#' | ';' if !in_single && !in_double && prev_was_whitespace => {
                return input[..idx].trim_end();
            }
            '/' if !in_single && !in_double && prev_was_whitespace => {
                if let Some((_, next)) = iter.peek() {
                    if *next == '/' {
                        return input[..idx].trim_end();
                    }
                }
                prev_was_whitespace = false;
            }
            ch if ch.is_whitespace() => {
                prev_was_whitespace = true;
            }
            _ => {
                prev_was_whitespace = false;
            }
        }
    }

    input
}

fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut brace_depth = 0_u32;
    let mut bracket_depth = 0_u32;
    let mut paren_depth = 0_u32;

    for (idx, ch) in input.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '{' if !in_single && !in_double => brace_depth += 1,
            '}' if !in_single && !in_double && brace_depth > 0 => brace_depth -= 1,
            '[' if !in_single && !in_double => bracket_depth += 1,
            ']' if !in_single && !in_double && bracket_depth > 0 => bracket_depth -= 1,
            '(' if !in_single && !in_double => paren_depth += 1,
            ')' if !in_single && !in_double && paren_depth > 0 => paren_depth -= 1,
            _ => {}
        }

        if ch == delimiter
            && !in_single
            && !in_double
            && brace_depth == 0
            && bracket_depth == 0
            && paren_depth == 0
        {
            let segment = input[start..idx].trim();
            if !segment.is_empty() {
                items.push(segment.to_string());
            }
            start = idx + ch.len_utf8();
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        items.push(tail.to_string());
    }

    items
}

fn shell_single_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

fn powershell_escape(input: &str) -> String {
    input.replace('`', "``").replace('"', "`\"")
}

fn py_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}

fn ruby_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}

fn is_js_identifier(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn strip_list_markers(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("- [ ] ")
                .or_else(|| line.strip_prefix("- "))
                .or_else(|| line.strip_prefix("* "))
                .or_else(|| line.strip_prefix("> "))
                .map(str::trim)
                .unwrap_or_else(|| strip_numbered_list_prefix(line).unwrap_or(line))
                .trim()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_numbered_list_prefix(line: &str) -> Option<&str> {
    let mut split_index = 0;
    for ch in line.chars() {
        if ch.is_ascii_digit() {
            split_index += ch.len_utf8();
            continue;
        }
        break;
    }

    if split_index == 0 {
        return None;
    }

    let suffix = line.get(split_index..)?;
    suffix
        .strip_prefix(". ")
        .or_else(|| suffix.strip_prefix(") "))
}

fn to_sentence_per_line(input: &str) -> String {
    input
        .split_inclusive(['.', '!', '?'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_markdown_heading(input: &str) -> String {
    let heading = input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if heading.is_empty() {
        "##".to_string()
    } else {
        format!("## {heading}")
    }
}

fn to_single_paragraph(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cleanup_spacing(input: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current_lines = Vec::new();

    for line in input.lines() {
        let normalized_line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized_line.is_empty() {
            if !current_lines.is_empty() {
                paragraphs.push(current_lines.join(" "));
                current_lines.clear();
            }
            continue;
        }
        current_lines.push(normalized_line);
    }

    if !current_lines.is_empty() {
        paragraphs.push(current_lines.join(" "));
    }

    paragraphs.join("\n\n")
}

fn polish_writing(input: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current_lines = Vec::new();

    for line in input.lines() {
        let normalized_line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized_line.is_empty() {
            if !current_lines.is_empty() {
                paragraphs.push(normalize_prose_paragraph(&current_lines.join("\n")));
                current_lines.clear();
            }
            continue;
        }

        current_lines.push(normalized_line);
    }

    if !current_lines.is_empty() {
        paragraphs.push(normalize_prose_paragraph(&current_lines.join("\n")));
    }

    if paragraphs.is_empty() {
        String::new()
    } else {
        paragraphs.join("\n\n")
    }
}

fn concise_rewrite(input: &str) -> String {
    let cleaned = cleanup_spacing(input);
    let mut rewritten_paragraphs = Vec::new();

    for paragraph in cleaned.split("\n\n") {
        let source_units = split_sentence_units(paragraph);
        let mut units = source_units
            .into_iter()
            .map(|unit| shorten_sentence(&unit))
            .filter(|unit| !unit.is_empty())
            .collect::<Vec<_>>();

        units = combine_question_alternative_units(units);

        if units.len() > 2 {
            units.truncate(2);
        }

        if !units.is_empty() {
            rewritten_paragraphs.push(units.join(" "));
        }
    }

    if rewritten_paragraphs.is_empty() {
        String::new()
    } else {
        rewritten_paragraphs.join("\n\n")
    }
}

fn combine_question_alternative_units(units: Vec<String>) -> Vec<String> {
    let mut combined = Vec::new();
    let mut index = 0;

    while index < units.len() {
        let alternative_index = if index + 2 < units.len() && units[index + 2].starts_with("Or ") {
            Some(index + 2)
        } else if index + 3 < units.len()
            && is_concise_discourse_question(&units[index + 2])
            && units[index + 3].starts_with("Or ")
        {
            Some(index + 3)
        } else {
            None
        };

        if units[index].starts_with("Can ")
            && index + 1 < units.len()
            && units[index + 1].contains("to check if ")
            && alternative_index.is_some()
        {
            let alternative_index = alternative_index.expect("checked above");
            let question = trim_terminal_punctuation(&units[index]);
            let purpose = trim_terminal_punctuation(&units[index + 1])
                .trim_start_matches("Just ")
                .trim_start_matches("just ")
                .trim_start_matches("To ")
                .trim_start_matches("to ");
            let alternative = normalize_alternative_question_phrase(
                trim_terminal_punctuation(&units[alternative_index])
                    .trim_start_matches("Or ")
                    .trim_start_matches("or "),
            );

            combined.push(normalize_sentence_for_prose(&format!(
                "{question} to {purpose}, or should we {alternative}"
            )));
            index = alternative_index + 1;
            continue;
        }

        combined.push(units[index].clone());
        index += 1;
    }

    combined
}

fn is_concise_discourse_question(input: &str) -> bool {
    matches!(
        trim_terminal_punctuation(input),
        "What do you think" | "What do you think?"
    )
}

fn normalize_alternative_question_phrase(input: &str) -> String {
    let mut output = input.trim();

    for prefix in [
        "we could just ",
        "We could just ",
        "we could ",
        "We could ",
        "we can just ",
        "We can just ",
        "we can ",
        "We can ",
        "just ",
        "Just ",
    ] {
        if let Some(stripped) = output.strip_prefix(prefix) {
            output = stripped.trim();
            break;
        }
    }

    output.to_string()
}

fn formal_rewrite(input: &str) -> String {
    let mut output = polish_writing(input);

    for (source, replacement) in [
        ("can't", "cannot"),
        ("won't", "will not"),
        ("don't", "do not"),
        ("doesn't", "does not"),
        ("didn't", "did not"),
        ("isn't", "is not"),
        ("aren't", "are not"),
        ("i'm", "I am"),
        ("we're", "we are"),
        ("you're", "you are"),
        ("it's", "it is"),
        ("that's", "that is"),
        ("there's", "there is"),
        ("pls", "please"),
        ("asap", "as soon as possible"),
        ("info", "information"),
        ("gonna", "going to"),
        ("wanna", "want to"),
    ] {
        output = replace_phrase_variants(output, source, replacement);
    }

    polish_writing(&output)
}

fn to_bullet_summary(input: &str) -> String {
    let mut units = split_sentence_units(&cleanup_spacing(input))
        .into_iter()
        .map(|unit| shorten_sentence(&unit))
        .filter(|unit| !unit.is_empty())
        .collect::<Vec<_>>();

    if units.is_empty() {
        return String::new();
    }

    if units.len() > 3 {
        units.truncate(3);
    }

    units
        .into_iter()
        .map(|unit| format!("- {}", trim_terminal_punctuation(&unit)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_prose_paragraph(input: &str) -> String {
    split_sentence_units(input)
        .into_iter()
        .map(|unit| normalize_sentence_for_prose(&unit))
        .filter(|unit| !unit.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_sentence_for_prose(input: &str) -> String {
    let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let chinese_terminal = trimmed
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .filter(|ch| matches!(ch, '\u{3002}' | '\u{ff01}' | '\u{ff1f}'));
    let body = trim_terminal_punctuation(trimmed);
    if body.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let mut capitalized = false;
    for ch in body.chars() {
        if !capitalized && ch.is_alphabetic() {
            output.extend(ch.to_uppercase());
            capitalized = true;
        } else {
            output.push(ch);
        }
    }

    if let Some(terminal) = chinese_terminal {
        output.push(terminal);
    } else if !output.ends_with(['.', '!', '?']) {
        output.push('.');
    }

    output
}

fn split_sentence_units(input: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        match ch {
            '\r' => {}
            '\n' => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    units.push(trimmed.to_string());
                    current.clear();
                }
            }
            '.' | '!' | '?' | '\u{3002}' | '\u{ff01}' | '\u{ff1f}' => {
                current.push(ch);
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    units.push(trimmed.to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        units.push(trimmed.to_string());
    }

    units
}

fn shorten_sentence(input: &str) -> String {
    let mut output = normalize_sentence_for_prose(input);

    output = strip_concise_leading_filler(output);
    output = remove_spoken_fillers(output);

    output = replace_phrase_variants(output, " in order to ", " to ");
    output = replace_phrase_variants(output, " due to the fact that ", " because ");
    output = replace_phrase_variants(output, " at this point in time ", " now ");
    output = replace_phrase_variants(output, " kind of ", " ");
    output = replace_phrase_variants(output, " sort of ", " ");
    output = replace_phrase_variants(output, " really ", " ");
    output = replace_phrase_variants(output, " very ", " ");
    output = replace_phrase_variants(output, " basically ", " ");
    output = replace_phrase_variants(output, " actually ", " ");
    output = replace_phrase_variants(output, " just ", " ");
    output = replace_phrase_variants(output, " a little bit ", " ");
    output = replace_phrase_variants(output, " a bit ", " ");
    output = replace_phrase_variants(output, ", and ", " and ");
    output = replace_phrase_variants(output, ", but ", " but ");
    output = replace_phrase_variants(output, ", so ", " so ");

    normalize_sentence_for_prose(&output)
}

fn strip_concise_leading_filler(mut output: String) -> String {
    while let Some(stripped) = [
        "Please note that I just wanted to ",
        "Please note that I wanted to ",
        "Please note that ",
        "I just wanted to ",
        "I wanted to ",
        "I think ",
        "I believe ",
        "Just ",
    ]
    .into_iter()
    .find_map(|prefix| output.strip_prefix(prefix))
    {
        output = normalize_sentence_for_prose(stripped);
    }

    output
}

fn remove_spoken_fillers(mut output: String) -> String {
    while let Some(stripped) = [
        "I mean, like, um, ",
        "I mean, like, ",
        "I mean, um, ",
        "I mean, ",
        "Like, ",
        "Um, ",
        "Uh, ",
    ]
    .into_iter()
    .find_map(|prefix| output.strip_prefix(prefix))
    {
        output = normalize_sentence_for_prose(stripped);
    }

    for filler in [
        " I mean, ",
        " i mean, ",
        " I mean ",
        " i mean ",
        " like, ",
        " Like, ",
        " like ",
        " Like ",
        " um, ",
        " Um, ",
        " um ",
        " Um ",
        " uh, ",
        " Uh, ",
        " uh ",
        " Uh ",
    ] {
        output = output.replace(filler, " ");
    }

    output = replace_phrase_variants(output, " I mean, like, um, ", " ");
    output = replace_phrase_variants(output, " I mean like um ", " ");
    cleanup_spacing(&output)
}

fn replace_phrase_variants(mut text: String, source: &str, replacement: &str) -> String {
    for (from, to) in [
        (source.to_string(), replacement.to_string()),
        (capitalize_phrase(source), capitalize_phrase(replacement)),
        (
            source.to_ascii_uppercase(),
            replacement.to_ascii_uppercase(),
        ),
    ] {
        if !from.is_empty() {
            text = text.replace(&from, &to);
        }
    }

    cleanup_spacing(&text)
}

fn capitalize_phrase(input: &str) -> String {
    let mut output = String::new();
    let mut capitalized = false;

    for ch in input.chars() {
        if !capitalized && ch.is_alphabetic() {
            output.extend(ch.to_uppercase());
            capitalized = true;
        } else {
            output.push(ch);
        }
    }

    output
}

fn trim_terminal_punctuation(input: &str) -> &str {
    input.trim_end_matches(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '.' | '!' | '?' | '\u{3002}' | '\u{ff01}' | '\u{ff1f}' | ',' | ';' | ':'
            )
    })
}

fn to_subject_line(input: &str) -> String {
    let trimmed = strip_email_request_prefix(input);

    if trimmed.is_empty() {
        "Follow-up".to_string()
    } else {
        to_title_case(trimmed)
    }
}

fn parse_email_request_details(input: &str) -> EmailDraftDetails {
    let trimmed = strip_email_request_prefix(input)
        .trim_end_matches(['.', '!', '?'])
        .trim();

    if trimmed.is_empty() {
        return EmailDraftDetails {
            recipient: None,
            topic: None,
            fallback_subject: "Follow-up".to_string(),
        };
    }

    if let Some(rest) = trimmed.strip_prefix("to ") {
        if let Some((recipient, topic)) = rest.split_once(" about ") {
            let clean_recipient = recipient.trim();
            let clean_topic = topic.trim();
            return EmailDraftDetails {
                recipient: (!clean_recipient.is_empty()).then(|| clean_recipient.to_string()),
                topic: (!clean_topic.is_empty()).then(|| clean_topic.to_string()),
                fallback_subject: to_subject_line(trimmed),
            };
        }

        let clean_recipient = rest.trim();
        return EmailDraftDetails {
            recipient: (!clean_recipient.is_empty()).then(|| clean_recipient.to_string()),
            topic: None,
            fallback_subject: to_subject_line(trimmed),
        };
    }

    EmailDraftDetails {
        recipient: None,
        topic: Some(trimmed.to_string()),
        fallback_subject: to_subject_line(trimmed),
    }
}

fn build_email_subject_line(details: &EmailDraftDetails) -> String {
    if let Some(topic) = &details.topic {
        return to_subject_line(topic);
    }

    if let Some(recipient) = &details.recipient {
        return format!("Follow-Up For {}", to_title_case(recipient));
    }

    details.fallback_subject.clone()
}

fn build_email_body(details: &EmailDraftDetails) -> String {
    match (&details.recipient, &details.topic) {
        (Some(recipient), Some(topic)) => format!(
            "I wanted to reach out to {} about {}.",
            recipient.trim_end_matches(['.', '!', '?']),
            topic.trim_end_matches(['.', '!', '?'])
        ),
        (Some(recipient), None) => format!(
            "I wanted to reach out to {} with a quick follow-up.",
            recipient.trim_end_matches(['.', '!', '?'])
        ),
        (None, Some(topic)) => format!(
            "I wanted to reach out about {}.",
            topic.trim_end_matches(['.', '!', '?'])
        ),
        (None, None) => {
            "I wanted to follow up and share a concise draft based on your request.".to_string()
        }
    }
}

struct EmailDraftDetails {
    recipient: Option<String>,
    topic: Option<String>,
    fallback_subject: String,
}

fn strip_email_request_prefix(input: &str) -> &str {
    input
        .trim()
        .trim_start_matches("help me write an email")
        .trim_start_matches("help me draft an email")
        .trim_start_matches("draft an email")
        .trim_start_matches("write an email")
        .trim_start_matches("email")
        .trim_matches(|ch: char| ch == ':' || ch == '-' || ch.is_whitespace())
}

fn build_reply_body(input: &str) -> String {
    let trimmed = input
        .trim()
        .trim_start_matches("reply to")
        .trim_start_matches("respond to")
        .trim_start_matches("write a reply to")
        .trim_matches(|ch: char| ch == ':' || ch == '-' || ch.is_whitespace());

    if trimmed.is_empty() {
        "Thanks for the note. I wanted to follow up with a concise reply.".to_string()
    } else if let Some((_, topic)) = trimmed.split_once(" about ") {
        format!(
            "Thanks for the update about {}. I wanted to send a quick reply and keep things moving.",
            topic.trim_end_matches(['.', '!', '?'])
        )
    } else {
        format!(
            "Thanks for the update about {}. I wanted to send a quick reply and keep things moving.",
            trimmed.trim_end_matches(['.', '!', '?'])
        )
    }
}

fn strip_checklist_prefix(input: &str) -> &str {
    input
        .trim()
        .trim_start_matches("make a checklist for")
        .trim_start_matches("make a checklist")
        .trim_start_matches("make a todo list for")
        .trim_start_matches("make a to do list for")
        .trim_start_matches("todo list for")
        .trim_start_matches("to do list for")
        .trim_start_matches("checklist for")
        .trim_matches(|ch: char| ch == ':' || ch == '-' || ch.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::{
        ProviderBackedSelectedTextExecutor, ProviderSelectedTextEditRequest,
        SelectedTextEditProvider, SelectedTextExecutor, TemporarySelectedTextExecutor,
        WakePhraseIntentExecutor,
    };
    use shared_protocol::{
        SelectedTextExecutionAction, SelectedTextExecutionRequest, WakePhraseIntentAction,
        WakePhraseIntentExecutionRequest,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct MockSelectedTextProvider {
        configured: bool,
        result: Result<String, String>,
        calls: Arc<AtomicUsize>,
    }

    impl MockSelectedTextProvider {
        fn succeeding(output: &str, calls: Arc<AtomicUsize>) -> Self {
            Self {
                configured: true,
                result: Ok(output.to_string()),
                calls,
            }
        }

        fn failing(error: &str, calls: Arc<AtomicUsize>) -> Self {
            Self {
                configured: true,
                result: Err(error.to_string()),
                calls,
            }
        }

        fn unconfigured(calls: Arc<AtomicUsize>) -> Self {
            Self {
                configured: false,
                result: Ok("provider should not be called".to_string()),
                calls,
            }
        }
    }

    impl SelectedTextEditProvider for MockSelectedTextProvider {
        fn profile_label(&self) -> &str {
            "BestQuality"
        }

        fn model_code(&self) -> &str {
            "deepseek-v4-flash"
        }

        fn is_configured(&self) -> bool {
            self.configured
        }

        fn edit(&self, request: &ProviderSelectedTextEditRequest<'_>) -> Result<String, String> {
            assert!(!request.selected_text.is_empty());
            assert!(!request.instruction_text.is_empty());
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result.clone()
        }
    }

    #[test]
    fn applies_uppercase_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 1,
                selected_text: "make me loud".to_string(),
                instruction_text: "uppercase".to_string(),
            })
            .expect("uppercase should be supported");

        assert_eq!(result.output_text, "MAKE ME LOUD");
        assert_eq!(result.action, SelectedTextExecutionAction::Uppercase);
        assert_eq!(result.normalized_instruction, "uppercase");
    }

    #[test]
    fn applies_prompt_scaffold() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 2,
                selected_text: "sort the rows by urgency".to_string(),
                instruction_text: "make this a prompt".to_string(),
            })
            .expect("prompt scaffold should be supported");

        assert!(result.output_text.contains("Write a polished prompt"));
        assert!(result.output_text.contains("sort the rows by urgency"));
    }

    #[test]
    fn polishes_selected_text_into_cleaner_prose() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 3,
                selected_text: "  this update needs cleanup   \n it should read better  "
                    .to_string(),
                instruction_text: "rewrite this".to_string(),
            })
            .expect("rewrite should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PolishWriting);
        assert_eq!(
            result.output_text,
            "This update needs cleanup. It should read better."
        );
    }

    #[test]
    fn rewrites_selected_text_more_concisely() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 4,
                selected_text: "Please note that I just wanted to share the rollout status, and we are basically ready to ship this change."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("concise rewrite should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(
            result.output_text,
            "Share the rollout status and we are ready to ship this change."
        );
    }

    #[test]
    fn concise_rewrite_preserves_meaningful_later_clauses() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 44,
                selected_text: "I wanted to explain the decision because the team needs context, but we can remove the extra background and keep the request focused."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("concise rewrite should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(
            result.output_text,
            "Explain the decision because the team needs context but we can remove the extra background and keep the request focused."
        );
    }

    #[test]
    fn concise_rewrite_removes_spoken_fillers_and_keeps_alternatives() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 45,
                selected_text: "Can we go there tomorrow? I mean, like, um, just to check if they are okay. What do you think? Or we could just call them."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("concise rewrite should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(
            result.output_text,
            "Can we go there tomorrow to check if they are okay, or should we call them."
        );
    }

    #[test]
    fn concise_rewrite_preserves_chinese_terminal_punctuation() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 46,
                selected_text: "请先检查 staged files。然后等我确认。".to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("concise rewrite should support Chinese punctuation");

        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(result.output_text, "请先检查 staged files。 然后等我确认。");
        assert!(!result.output_text.contains("。."));
        assert!(!result.output_text.ends_with("。."));
    }

    #[test]
    fn concise_rewrite_treats_chinese_punctuation_as_sentence_boundaries() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 47,
                selected_text: "第一句保留。第二句也保留！第三句应该被截断？".to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("concise rewrite should split Chinese sentence units");

        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(result.output_text, "第一句保留。 第二句也保留！");
        assert!(!result.output_text.contains("第三句"));
    }

    #[test]
    fn rewrites_selected_text_in_a_more_formal_tone() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 5,
                selected_text: "i'm gonna send the info asap, but we can't promise a date yet."
                    .to_string(),
                instruction_text: "make this more professional".to_string(),
            })
            .expect("formal rewrite should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::FormalRewrite);
        assert_eq!(
            result.output_text,
            "I am going to send the information as soon as possible, but we cannot promise a date yet."
        );
    }

    #[test]
    fn summarizes_selected_text_into_bullets() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 6,
                selected_text: "The host now mirrors runtime state to the companion surfaces. The selected-text path also reports executor metadata. We still need broader app compatibility validation."
                    .to_string(),
                instruction_text: "summarize this".to_string(),
            })
            .expect("summary should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::BulletSummary);
        assert_eq!(
            result.output_text,
            "- The host now mirrors runtime state to the companion surfaces\n- The selected-text path also reports executor metadata\n- We still need broader app compatibility validation"
        );
    }

    #[test]
    fn provider_backed_rewrite_success_replaces_deterministic_output() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::succeeding(
                "Let Codex list staged files, then commit after confirmation.",
                Arc::clone(&calls),
            ),
        );

        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 80,
                selected_text: "Please note that Codex should probably show the staged files before committing."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("provider-backed concise rewrite should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.action, SelectedTextExecutionAction::ConciseRewrite);
        assert_eq!(
            result.output_text,
            "Let Codex list staged files, then commit after confirmation."
        );
        let diagnostics = result
            .provider_diagnostics
            .expect("provider diagnostics should be present");
        assert!(diagnostics.provider_attempted);
        assert!(diagnostics.provider_succeeded);
        assert!(!diagnostics.deterministic_fallback_used);
        assert_eq!(diagnostics.model_code, "deepseek-v4-flash");
        assert_eq!(
            diagnostics.output_char_count,
            result.output_text.chars().count()
        );
    }

    #[test]
    fn provider_backed_rewrite_failure_falls_back_to_deterministic_output() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::failing("timeout", Arc::clone(&calls)),
        );

        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 81,
                selected_text: "Please note that I just wanted to share the rollout status, and we are basically ready to ship this change."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("provider failure should fall back deterministically");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.output_text,
            "Share the rollout status and we are ready to ship this change."
        );
        let diagnostics = result
            .provider_diagnostics
            .expect("provider diagnostics should be present");
        assert!(diagnostics.provider_attempted);
        assert!(!diagnostics.provider_succeeded);
        assert!(diagnostics.deterministic_fallback_used);
        assert!(
            diagnostics
                .fallback_reason
                .expect("fallback reason should be present")
                .contains("timeout")
        );
    }

    #[test]
    fn missing_selected_text_provider_config_falls_back_without_calling_provider() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::unconfigured(Arc::clone(&calls)),
        );

        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 82,
                selected_text: "Please note that I just wanted to share the rollout status, and we are basically ready to ship this change."
                    .to_string(),
                instruction_text: "make this more concise".to_string(),
            })
            .expect("missing provider config should use deterministic fallback");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            result.output_text,
            "Share the rollout status and we are ready to ship this change."
        );
        let diagnostics = result
            .provider_diagnostics
            .expect("provider diagnostics should be present");
        assert!(!diagnostics.provider_attempted);
        assert!(!diagnostics.provider_succeeded);
        assert!(diagnostics.deterministic_fallback_used);
    }

    #[test]
    fn structural_selected_text_action_does_not_call_provider() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::succeeding("provider should not win", Arc::clone(&calls)),
        );

        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 83,
                selected_text: "make me loud".to_string(),
                instruction_text: "uppercase".to_string(),
            })
            .expect("uppercase should stay deterministic");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(result.output_text, "MAKE ME LOUD");
        assert_eq!(result.action, SelectedTextExecutionAction::Uppercase);
        assert_eq!(result.provider_diagnostics, None);
    }

    #[test]
    fn provider_backed_selected_text_routes_make_it_more_concise_to_provider_edit() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::succeeding("Short provider rewrite.", Arc::clone(&calls)),
        );

        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 84,
                selected_text: "This paragraph can be shorter and clearer.".to_string(),
                instruction_text: "make it more concise".to_string(),
            })
            .expect("freeform concise command should route through provider");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.action,
            SelectedTextExecutionAction::GeneralProviderEdit
        );
        assert_eq!(result.output_text, "Short provider rewrite.");
        assert!(result.provider_diagnostics.unwrap().provider_succeeded);
    }

    #[test]
    fn provider_backed_selected_text_routes_multilingual_unknown_instructions_to_general_provider_edit()
     {
        let instructions = [
            "translate it into English",
            "翻译成英文",
            "帮我翻译成德语",
            "把它改成 vibe coding 的命令",
            "写成商务邮件",
            "帮我优化这段 prompt",
            "英語に翻訳して",
            "더 간결하게 해줘",
            "幫我改短啲",
        ];

        for instruction in instructions {
            let calls = Arc::new(AtomicUsize::new(0));
            let executor = ProviderBackedSelectedTextExecutor::new(
                TemporarySelectedTextExecutor,
                MockSelectedTextProvider::succeeding("provider freeform edit", Arc::clone(&calls)),
            );

            let result = executor
                .execute(&SelectedTextExecutionRequest {
                    session_id: 85,
                    selected_text: "Selected text to edit.".to_string(),
                    instruction_text: instruction.to_string(),
                })
                .expect("unknown selected-text instruction should use provider");

            assert_eq!(calls.load(Ordering::SeqCst), 1, "{instruction}");
            assert_eq!(
                result.action,
                SelectedTextExecutionAction::GeneralProviderEdit,
                "{instruction}"
            );
            assert_eq!(result.output_text, "provider freeform edit");
            let diagnostics = result
                .provider_diagnostics
                .expect("provider diagnostics should be present");
            assert!(diagnostics.provider_attempted);
            assert!(diagnostics.provider_succeeded);
            assert!(!diagnostics.deterministic_fallback_used);
        }
    }

    #[test]
    fn provider_backed_general_edit_requires_configured_provider() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::unconfigured(Arc::clone(&calls)),
        );

        let error = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 86,
                selected_text: "Selected text to edit.".to_string(),
                instruction_text: "translate it into English".to_string(),
            })
            .expect_err("freeform edit should fail clearly without provider config");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(error.contains("freeform edit requires provider"));
    }

    #[test]
    fn provider_backed_general_edit_does_not_use_poor_deterministic_fallback_on_provider_failure() {
        let calls = Arc::new(AtomicUsize::new(0));
        let executor = ProviderBackedSelectedTextExecutor::new(
            TemporarySelectedTextExecutor,
            MockSelectedTextProvider::failing("timeout", Arc::clone(&calls)),
        );

        let error = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 87,
                selected_text: "Selected text to edit.".to_string(),
                instruction_text: "帮我翻译成德语".to_string(),
            })
            .expect_err("provider failure should fail clearly for freeform edit");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(error.contains("freeform edit failed"));
        assert!(error.contains("timeout"));
    }

    #[test]
    fn applies_snake_case_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 15,
                selected_text: "Sort These Rows".to_string(),
                instruction_text: "snake case".to_string(),
            })
            .expect("snake case should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SnakeCase);
        assert_eq!(result.output_text, "sort_these_rows");
    }

    #[test]
    fn applies_kebab_case_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 16,
                selected_text: "Sort These Rows".to_string(),
                instruction_text: "kebab case".to_string(),
            })
            .expect("kebab case should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::KebabCase);
        assert_eq!(result.output_text, "sort-these-rows");
    }

    #[test]
    fn applies_camel_case_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 17,
                selected_text: "Sort These Rows".to_string(),
                instruction_text: "camel case".to_string(),
            })
            .expect("camel case should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::CamelCase);
        assert_eq!(result.output_text, "sortTheseRows");
    }

    #[test]
    fn applies_pascal_case_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 18,
                selected_text: "Sort These Rows".to_string(),
                instruction_text: "pascal case".to_string(),
            })
            .expect("pascal case should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PascalCase);
        assert_eq!(result.output_text, "SortTheseRows");
    }

    #[test]
    fn applies_constant_case_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 19,
                selected_text: "Sort These Rows".to_string(),
                instruction_text: "constant case".to_string(),
            })
            .expect("constant case should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::ConstantCase);
        assert_eq!(result.output_text, "SORT_THESE_ROWS");
    }

    #[test]
    fn applies_inline_code_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 27,
                selected_text: " launch checklist ".to_string(),
                instruction_text: "inline code".to_string(),
            })
            .expect("inline code should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::InlineCode);
        assert_eq!(result.output_text, "`launch checklist`");
    }

    #[test]
    fn applies_code_block_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 28,
                selected_text: "alpha\nbeta".to_string(),
                instruction_text: "code block".to_string(),
            })
            .expect("code block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::CodeBlock);
        assert_eq!(result.output_text, "```\nalpha\nbeta\n```");
    }

    #[test]
    fn strips_code_fence_markers() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 29,
                selected_text: "```rust\nalpha\nbeta\n```".to_string(),
                instruction_text: "strip code fence".to_string(),
            })
            .expect("strip code fence should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::StripCodeFence);
        assert_eq!(result.output_text, "alpha\nbeta");
    }

    #[test]
    fn applies_markdown_bold_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 30,
                selected_text: " launch summary ".to_string(),
                instruction_text: "make this bold".to_string(),
            })
            .expect("markdown bold should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::MarkdownBold);
        assert_eq!(result.output_text, "**launch summary**");
    }

    #[test]
    fn applies_markdown_italic_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 31,
                selected_text: " launch summary ".to_string(),
                instruction_text: "markdown italic".to_string(),
            })
            .expect("markdown italic should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::MarkdownItalic);
        assert_eq!(result.output_text, "*launch summary*");
    }

    #[test]
    fn strips_markdown_emphasis_markers() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 32,
                selected_text: "**alpha**\n_beta_\n__gamma__".to_string(),
                instruction_text: "strip markdown emphasis".to_string(),
            })
            .expect("strip markdown emphasis should be supported");

        assert_eq!(
            result.action,
            SelectedTextExecutionAction::StripMarkdownEmphasis
        );
        assert_eq!(result.output_text, "alpha\nbeta\ngamma");
    }

    #[test]
    fn applies_numbered_list_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 10,
                selected_text: "first line\nsecond line\nthird line".to_string(),
                instruction_text: "make this a numbered list".to_string(),
            })
            .expect("numbered list should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::NumberedList);
        assert_eq!(
            result.output_text,
            "1. first line\n2. second line\n3. third line"
        );
    }

    #[test]
    fn applies_checklist_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 21,
                selected_text: "first line\nsecond line".to_string(),
                instruction_text: "make this a checklist".to_string(),
            })
            .expect("checklist should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::Checklist);
        assert_eq!(result.output_text, "- [ ] first line\n- [ ] second line");
    }

    #[test]
    fn applies_quote_block_formatting() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 22,
                selected_text: "first line\n second line ".to_string(),
                instruction_text: "quote block".to_string(),
            })
            .expect("quote block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::QuoteBlock);
        assert_eq!(result.output_text, "> first line\n> second line");
    }

    #[test]
    fn sorts_lines_alphabetically() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 13,
                selected_text: "pear\napple\nbanana".to_string(),
                instruction_text: "sort lines".to_string(),
            })
            .expect("sort lines should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SortLines);
        assert_eq!(result.output_text, "apple\nbanana\npear");
    }

    #[test]
    fn removes_duplicate_lines_while_preserving_first_occurrence_order() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 14,
                selected_text: "alpha\nbeta\nalpha\nBeta\nbeta".to_string(),
                instruction_text: "remove duplicate lines".to_string(),
            })
            .expect("deduplicate lines should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::DeduplicateLines);
        assert_eq!(result.output_text, "alpha\nbeta");
    }

    #[test]
    fn removes_empty_lines_while_preserving_content_order() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 20,
                selected_text: "alpha\n\n beta \n \nGamma".to_string(),
                instruction_text: "remove empty lines".to_string(),
            })
            .expect("remove empty lines should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::RemoveEmptyLines);
        assert_eq!(result.output_text, "alpha\nbeta\nGamma");
    }

    #[test]
    fn joins_selection_into_comma_separated_text() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 23,
                selected_text: "alpha\nbeta, gamma\n delta ".to_string(),
                instruction_text: "comma separated".to_string(),
            })
            .expect("comma separated should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::CommaSeparated);
        assert_eq!(result.output_text, "alpha, beta, gamma, delta");
    }

    #[test]
    fn joins_selection_into_pipe_separated_text() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 33,
                selected_text: "alpha\nbeta, gamma\n delta ".to_string(),
                instruction_text: "pipe separated".to_string(),
            })
            .expect("pipe separated should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PipeSeparated);
        assert_eq!(result.output_text, "alpha | beta | gamma | delta");
    }

    #[test]
    fn joins_selection_into_tab_separated_text() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 34,
                selected_text: "alpha\nbeta, gamma\n delta ".to_string(),
                instruction_text: "tab separated".to_string(),
            })
            .expect("tab separated should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::TabSeparated);
        assert_eq!(result.output_text, "alpha\tbeta\tgamma\tdelta");
    }

    #[test]
    fn converts_selection_into_json_array() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 35,
                selected_text: "alpha\nbeta, gamma\n delta ".to_string(),
                instruction_text: "json array".to_string(),
            })
            .expect("json array should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonArray);
        assert_eq!(
            result.output_text,
            "[\n  \"alpha\",\n  \"beta\",\n  \"gamma\",\n  \"delta\"\n]"
        );
    }

    #[test]
    fn joins_selection_into_semicolon_separated_text() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 36,
                selected_text: "alpha\nbeta, gamma\n delta ".to_string(),
                instruction_text: "semicolon separated".to_string(),
            })
            .expect("semicolon separated should be supported");

        assert_eq!(
            result.action,
            SelectedTextExecutionAction::SemicolonSeparated
        );
        assert_eq!(result.output_text, "alpha; beta; gamma; delta");
    }

    #[test]
    fn converts_selection_into_quoted_csv_text() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 37,
                selected_text: "alpha\nbeta \"quoted\", gamma".to_string(),
                instruction_text: "quoted csv".to_string(),
            })
            .expect("quoted csv should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::QuotedCsv);
        assert_eq!(
            result.output_text,
            "\"alpha\", \"beta \"\"quoted\"\"\", \"gamma\""
        );
    }

    #[test]
    fn converts_selection_into_sql_in_list() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 38,
                selected_text: "alpha\ncan't, gamma".to_string(),
                instruction_text: "sql in list".to_string(),
            })
            .expect("sql in list should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SqlInList);
        assert_eq!(result.output_text, "('alpha', 'can''t', 'gamma')");
    }

    #[test]
    fn converts_selection_into_yaml_list() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 39,
                selected_text: "alpha\nneeds: quote, gamma".to_string(),
                instruction_text: "yaml list".to_string(),
            })
            .expect("yaml list should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::YamlList);
        assert_eq!(result.output_text, "- alpha\n- \"needs: quote\"\n- gamma");
    }

    #[test]
    fn converts_selection_into_yaml_mapping() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 40,
                selected_text: "host=example.com\nneeds quote: alpha: beta".to_string(),
                instruction_text: "yaml mapping".to_string(),
            })
            .expect("yaml mapping should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::YamlMapping);
        assert_eq!(
            result.output_text,
            "host: example.com\n\"needs quote\": \"alpha: beta\""
        );
    }

    #[test]
    fn converts_selection_into_markdown_table() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 41,
                selected_text: "alpha\nbeta | gamma".to_string(),
                instruction_text: "markdown table".to_string(),
            })
            .expect("markdown table should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::MarkdownTable);
        assert_eq!(
            result.output_text,
            "| Value |\n| --- |\n| alpha |\n| beta \\| gamma |"
        );
    }

    #[test]
    fn converts_selection_into_header_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 42,
                selected_text: "Host=example.com\nAuthorization: Bearer abc\nX-Flag".to_string(),
                instruction_text: "header block".to_string(),
            })
            .expect("header block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::HeaderBlock);
        assert_eq!(
            result.output_text,
            "Host: example.com\nAuthorization: Bearer abc\nX-Flag:"
        );
    }

    #[test]
    fn converts_selection_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 43,
                selected_text: "host=example.com\nAuthorization: Bearer abc".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"Authorization\": \"Bearer abc\"\n}"
        );
    }

    #[test]
    fn preserves_typed_unquoted_scalars_in_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 44,
                selected_text: "enabled=true\nretries=3\nratio=1.5\nmissing=null".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"enabled\": true,\n  \"retries\": 3,\n  \"ratio\": 1.5,\n  \"missing\": null\n}"
        );
    }

    #[test]
    fn preserves_quoted_scalars_as_strings_in_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 45,
                selected_text: "enabled='true'\nretries=\"3\"\nmissing='null'".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"enabled\": \"true\",\n  \"retries\": \"3\",\n  \"missing\": \"null\"\n}"
        );
    }

    #[test]
    fn preserves_typed_unquoted_scalars_in_python_dict() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 46,
                selected_text: "enabled=true\nretries=3\nmissing=null".to_string(),
                instruction_text: "python dict".to_string(),
            })
            .expect("python dict should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PythonDict);
        assert_eq!(
            result.output_text,
            "{\n    'enabled': True,\n    'retries': 3,\n    'missing': None\n}"
        );
    }

    #[test]
    fn preserves_typed_unquoted_scalars_in_toml_table() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 47,
                selected_text: "enabled=true\nport=8080\nlabel='8080'\nmissing=null".to_string(),
                instruction_text: "toml table".to_string(),
            })
            .expect("toml table should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::TomlTable);
        assert_eq!(
            result.output_text,
            "enabled = true\nport = 8080\nlabel = \"8080\"\nmissing = \"null\""
        );
    }

    #[test]
    fn converts_selection_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 48,
                selected_text: "api key=secret token\nlog level=debug mode".to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(
            result.output_text,
            "API_KEY=\"secret token\"\nLOG_LEVEL=\"debug mode\""
        );
    }

    #[test]
    fn converts_selection_into_query_string() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 49,
                selected_text: "search=hello world\npage=1".to_string(),
                instruction_text: "query string".to_string(),
            })
            .expect("query string should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::QueryString);
        assert_eq!(result.output_text, "search=hello+world&page=1");
    }

    #[test]
    fn converts_selection_into_toml_table() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 50,
                selected_text: "api-key=secret token\nneeds quote=value".to_string(),
                instruction_text: "toml table".to_string(),
            })
            .expect("toml table should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::TomlTable);
        assert_eq!(
            result.output_text,
            "api-key = \"secret token\"\n\"needs quote\" = \"value\""
        );
    }

    #[test]
    fn converts_selection_into_shell_exports() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 47,
                selected_text: "api key=secret token\nowner=O'Reilly".to_string(),
                instruction_text: "shell exports".to_string(),
            })
            .expect("shell exports should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::ShellExports);
        assert_eq!(
            result.output_text,
            "export API_KEY='secret token'\nexport OWNER='O'\"'\"'Reilly'"
        );
    }

    #[test]
    fn converts_selection_into_powershell_env_assignments() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 48,
                selected_text: "api key=secret token\nquote=He said \"hi\"".to_string(),
                instruction_text: "powershell env".to_string(),
            })
            .expect("powershell env should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PowershellEnv);
        assert_eq!(
            result.output_text,
            "$env:API_KEY = \"secret token\"\n$env:QUOTE = \"He said `\"hi`\"\""
        );
    }

    #[test]
    fn converts_selection_into_curl_headers() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 49,
                selected_text: "Authorization=Bearer abc\nX-Trace=trace 1".to_string(),
                instruction_text: "curl headers".to_string(),
            })
            .expect("curl headers should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::CurlHeaders);
        assert_eq!(
            result.output_text,
            "-H 'Authorization: Bearer abc' \\\n-H 'X-Trace: trace 1'"
        );
    }

    #[test]
    fn converts_curl_command_line_into_yaml_mapping() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 72,
                selected_text:
                    "curl https://api.example.com -H 'Authorization: Bearer abc' --header \"X-Trace: trace 1\""
                        .to_string(),
                instruction_text: "yaml mapping".to_string(),
            })
            .expect("yaml mapping should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::YamlMapping);
        assert_eq!(
            result.output_text,
            "Authorization: Bearer abc\nX-Trace: trace 1"
        );
    }

    #[test]
    fn converts_curl_data_query_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 74,
                selected_text: "curl https://api.example.com --data 'host=example.com&page=1'"
                    .to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"page\": 1\n}"
        );
    }

    #[test]
    fn converts_curl_data_raw_json_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 75,
                selected_text:
                    "curl https://api.example.com --data-raw '{\"host\":\"example.com\",\"page\":\"1\"}'"
                        .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(result.output_text, "HOST=example.com\nPAGE=1");
    }

    #[test]
    fn converts_selection_into_python_dict() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 50,
                selected_text: "host=example.com\nowner=O'Reilly".to_string(),
                instruction_text: "python dict".to_string(),
            })
            .expect("python dict should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PythonDict);
        assert_eq!(
            result.output_text,
            "{\n    'host': 'example.com',\n    'owner': 'O\\'Reilly'\n}"
        );
    }

    #[test]
    fn converts_hashrocket_pairs_into_python_dict() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 51,
                selected_text: "host => example.com\nowner => O'Reilly".to_string(),
                instruction_text: "python dict".to_string(),
            })
            .expect("python dict should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::PythonDict);
        assert_eq!(
            result.output_text,
            "{\n    'host': 'example.com',\n    'owner': 'O\\'Reilly'\n}"
        );
    }

    #[test]
    fn converts_json_object_snippet_into_query_string() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 52,
                selected_text: "{\n  \"host\": \"example.com\",\n  \"page\": \"1\",\n}".to_string(),
                instruction_text: "query string".to_string(),
            })
            .expect("query string should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::QueryString);
        assert_eq!(result.output_text, "host=example.com&page=1");
    }

    #[test]
    fn converts_single_line_json_object_into_query_string() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 68,
                selected_text: "{\"host\":\"example.com\",\"page\":\"1\"}".to_string(),
                instruction_text: "query string".to_string(),
            })
            .expect("query string should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::QueryString);
        assert_eq!(result.output_text, "host=example.com&page=1");
    }

    #[test]
    fn converts_single_line_javascript_object_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 69,
                selected_text: "{ host: \"example.com\", page: \"1\" }".to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(result.output_text, "HOST=example.com\nPAGE=1");
    }

    #[test]
    fn converts_single_line_powershell_hashtable_into_yaml_mapping() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 77,
                selected_text: "@{ Authorization = \"Bearer abc\"; XTrace = \"trace 1\" }"
                    .to_string(),
                instruction_text: "yaml mapping".to_string(),
            })
            .expect("yaml mapping should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::YamlMapping);
        assert_eq!(
            result.output_text,
            "Authorization: Bearer abc\nXTrace: trace 1"
        );
    }

    #[test]
    fn converts_spaced_query_string_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 70,
                selected_text: "host=example.com & page=1".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"page\": 1\n}"
        );
    }

    #[test]
    fn converts_semicolon_separated_pairs_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 78,
                selected_text: "host=example.com; page=1; api key=secret token".to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(
            result.output_text,
            "HOST=example.com\nPAGE=1\nAPI_KEY=\"secret token\""
        );
    }

    #[test]
    fn converts_url_query_string_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 73,
                selected_text: "https://example.com/search?host=example.com&page=1#results"
                    .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(result.output_text, "HOST=example.com\nPAGE=1");
    }

    #[test]
    fn converts_quoted_curl_url_query_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 76,
                selected_text:
                    "curl \"https://example.com/search?host=example.com&page=1#results\""
                        .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(result.output_text, "HOST=example.com\nPAGE=1");
    }

    #[test]
    fn preserves_ampersands_inside_query_values() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 71,
                selected_text: "title=R&D".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(result.output_text, "{\n  \"title\": \"R&D\"\n}");
    }

    #[test]
    fn converts_commented_dotenv_snippet_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 53,
                selected_text: "# comment\nAPI_KEY=secret token # keep local\nLOG_LEVEL=debug"
                    .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(
            result.output_text,
            "API_KEY=\"secret token\"\nLOG_LEVEL=debug"
        );
    }

    #[test]
    fn converts_selection_into_javascript_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 54,
                selected_text: "host=example.com\nneeds quote=alpha beta".to_string(),
                instruction_text: "javascript object".to_string(),
            })
            .expect("javascript object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JavascriptObject);
        assert_eq!(
            result.output_text,
            "{\n  host: \"example.com\",\n  \"needs quote\": \"alpha beta\"\n}"
        );
    }

    #[test]
    fn converts_javascript_object_snippet_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 55,
                selected_text: "{\n  host: \"example.com\",\n  apiKey: \"secret token\",\n}"
                    .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(
            result.output_text,
            "HOST=example.com\nAPIKEY=\"secret token\""
        );
    }

    #[test]
    fn converts_commented_javascript_object_snippet_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 56,
                selected_text: "{\n  host: \"example.com\", // primary\n  page: \"1\"\n}"
                    .to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"page\": \"1\"\n}"
        );
    }

    #[test]
    fn converts_export_lines_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 57,
                selected_text: "export HOST=example.com\nexport OWNER='O\\'Reilly'".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"HOST\": \"example.com\",\n  \"OWNER\": \"O\\\\'Reilly\"\n}"
        );
    }

    #[test]
    fn converts_selection_into_ruby_hash() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 58,
                selected_text: "host=example.com\nowner=O'Reilly".to_string(),
                instruction_text: "ruby hash".to_string(),
            })
            .expect("ruby hash should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::RubyHash);
        assert_eq!(
            result.output_text,
            "{\n  'host' => 'example.com',\n  'owner' => 'O\\'Reilly'\n}"
        );
    }

    #[test]
    fn converts_powershell_env_assignments_into_env_block() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 59,
                selected_text: "$env:API_KEY = \"secret token\"\n$env:LOG_LEVEL = debug"
                    .to_string(),
                instruction_text: "env block".to_string(),
            })
            .expect("env block should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::EnvBlock);
        assert_eq!(
            result.output_text,
            "API_KEY=\"secret token\"\nLOG_LEVEL=debug"
        );
    }

    #[test]
    fn converts_curl_header_flags_into_yaml_mapping() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 60,
                selected_text: "-H 'Authorization: Bearer abc'\n-H 'X-Trace: trace 1'".to_string(),
                instruction_text: "yaml mapping".to_string(),
            })
            .expect("yaml mapping should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::YamlMapping);
        assert_eq!(
            result.output_text,
            "Authorization: Bearer abc\nX-Trace: trace 1"
        );
    }

    #[test]
    fn converts_commented_toml_snippet_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 61,
                selected_text: "[server]\nhost = \"example.com\" # prod\nport = \"8080\""
                    .to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"port\": \"8080\"\n}"
        );
    }

    #[test]
    fn converts_toml_table_snippet_into_json_object() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 62,
                selected_text: "[server]\nhost = \"example.com\"\nport = \"8080\"".to_string(),
                instruction_text: "json object".to_string(),
            })
            .expect("json object should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::JsonObject);
        assert_eq!(
            result.output_text,
            "{\n  \"host\": \"example.com\",\n  \"port\": \"8080\"\n}"
        );
    }

    #[test]
    fn converts_selection_into_sql_values_rows() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 63,
                selected_text: "alpha\ncan't, gamma".to_string(),
                instruction_text: "sql values rows".to_string(),
            })
            .expect("sql values rows should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SqlValuesRows);
        assert_eq!(result.output_text, "('alpha'),\n('can''t'),\n('gamma')");
    }

    #[test]
    fn strips_common_list_markers() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 24,
                selected_text: "- [ ] alpha\n- beta\n1. gamma\n> delta".to_string(),
                instruction_text: "strip list markers".to_string(),
            })
            .expect("strip list markers should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::StripListMarkers);
        assert_eq!(result.output_text, "alpha\nbeta\ngamma\ndelta");
    }

    #[test]
    fn splits_selection_into_sentence_per_line() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 25,
                selected_text: "Alpha is ready. Beta follows! Gamma closes?".to_string(),
                instruction_text: "sentence per line".to_string(),
            })
            .expect("sentence per line should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SentencePerLine);
        assert_eq!(
            result.output_text,
            "Alpha is ready.\nBeta follows!\nGamma closes?"
        );
    }

    #[test]
    fn reformats_selection_as_markdown_heading() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 26,
                selected_text: " launch   checklist overview ".to_string(),
                instruction_text: "markdown heading".to_string(),
            })
            .expect("markdown heading should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::MarkdownHeading);
        assert_eq!(result.output_text, "## launch checklist overview");
    }

    #[test]
    fn collapses_selection_into_single_paragraph() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 11,
                selected_text: "first line\nsecond   line\n\nthird line".to_string(),
                instruction_text: "remove line breaks".to_string(),
            })
            .expect("single paragraph should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::SingleParagraph);
        assert_eq!(result.output_text, "first line second line third line");
    }

    #[test]
    fn cleans_up_spacing_while_preserving_paragraph_breaks() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 12,
                selected_text: " first   line \n second line \n\n third   paragraph ".to_string(),
                instruction_text: "clean up spacing".to_string(),
            })
            .expect("spacing cleanup should be supported");

        assert_eq!(result.action, SelectedTextExecutionAction::CleanupSpacing);
        assert_eq!(
            result.output_text,
            "first line second line\n\nthird paragraph"
        );
    }

    #[test]
    fn rejects_unknown_instruction() {
        let executor = TemporarySelectedTextExecutor;
        let error = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 3,
                selected_text: "hello".to_string(),
                instruction_text: "translate this into English".to_string(),
            })
            .expect_err("unsupported instructions should fail explicitly");

        assert!(error.contains("does not support instruction"));
    }

    #[test]
    fn does_not_match_accidental_substrings_inside_other_words() {
        let executor = TemporarySelectedTextExecutor;
        let error = executor
            .execute(&SelectedTextExecutionRequest {
                session_id: 4,
                selected_text: "make me quiet".to_string(),
                instruction_text: "slower case slower case".to_string(),
            })
            .expect_err("substring matches should not trigger lowercase formatting");

        assert!(error.contains("does not support instruction"));
    }

    #[test]
    fn expands_email_requests_into_email_drafts() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute_intent(&WakePhraseIntentExecutionRequest {
                session_id: 5,
                command_text: "draft an email to my manager about taking Friday off".to_string(),
            })
            .expect("email draft should be supported");

        assert_eq!(result.action, WakePhraseIntentAction::DraftEmail);
        assert!(result.output_text.contains("Subject: Taking Friday Off"));
        assert!(result.output_text.contains("Hi,"));
        assert!(
            result
                .output_text
                .contains("reach out to my manager about taking Friday off")
        );
    }

    #[test]
    fn falls_back_to_general_draft_for_unknown_intent_requests() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute_intent(&WakePhraseIntentExecutionRequest {
                session_id: 6,
                command_text: "help me think through this pitch".to_string(),
            })
            .expect("general draft fallback should be available");

        assert_eq!(result.action, WakePhraseIntentAction::GeneralDraft);
        assert!(result.output_text.contains("Draft:"));
    }

    #[test]
    fn expands_reply_requests_into_reply_drafts() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute_intent(&WakePhraseIntentExecutionRequest {
                session_id: 7,
                command_text: "reply to the client about the revised timeline".to_string(),
            })
            .expect("reply draft should be supported");

        assert_eq!(result.action, WakePhraseIntentAction::ReplyMessage);
        assert!(result.output_text.contains("Hi,"));
        assert!(result.output_text.contains("Thanks for the update"));
    }

    #[test]
    fn expands_checklist_requests_into_bulleted_steps() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute_intent(&WakePhraseIntentExecutionRequest {
                session_id: 8,
                command_text: "make a checklist for launching the beta".to_string(),
            })
            .expect("checklist scaffold should be supported");

        assert_eq!(result.action, WakePhraseIntentAction::Checklist);
        assert!(
            result
                .output_text
                .contains("- Clarify the requested outcome")
        );
        assert!(result.output_text.contains("launching the beta"));
    }

    #[test]
    fn expands_email_requests_without_topic_into_a_recipient_follow_up() {
        let executor = TemporarySelectedTextExecutor;
        let result = executor
            .execute_intent(&WakePhraseIntentExecutionRequest {
                session_id: 9,
                command_text: "draft an email to finance".to_string(),
            })
            .expect("recipient-only email draft should be supported");

        assert_eq!(result.action, WakePhraseIntentAction::DraftEmail);
        assert!(
            result
                .output_text
                .contains("Subject: Follow-Up For Finance")
        );
        assert!(
            result
                .output_text
                .contains("reach out to finance with a quick follow-up")
        );
    }
}
