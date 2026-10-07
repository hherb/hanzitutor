//! Asking a model to check a sentence the learner wrote, and what the app will and
//! will not do with the answer.
//!
//! N24 adds two things to the composition exercise: the app can **speak** the
//! learner's own sentence back (`nihongo_core::composition::spoken_form`), and it can
//! **ask a model** whether the sentence is right. This module is the second one.
//!
//! # It is opt-in, off by default, and the app still fetches nothing by itself
//!
//! `HANDOVER_NIHONGO.md` invariant 43 is the rule, and it is a deliberate amendment
//! of this crate's oldest promise. Until N24 there was **no network path in this
//! crate at all**; there is one now, and it is:
//!
//! * **off until the learner configures it** — the endpoint, the model name and
//!   (where the endpoint needs one) the key are the learner's, in a fifth… sixth file
//!   of this app's own, and an unconfigured app has no address to reach;
//! * **only ever a request the learner pressed for** — nothing here runs on startup,
//!   on a timer, or as a side effect of writing a sentence;
//! * **of two things and no more**: the **pattern** the learner is looking at (and its
//!   kana reading where the corpus stores one) and **the learner's own sentence**.
//!   None of the corpus's explanations, formation, notes or example sentences is
//!   sent, because those are OpenJLPT's text under CC BY-SA 4.0 and this app does not
//!   hand a corpus to a third party to check one sentence;
//! * **never recorded** — the model's answer is shown and forgotten. It does not enter
//!   `compositions.json`, it does not move the pattern's weight, and it never joins
//!   the checklist `nihongo_core::composition` builds. The app can say what it saw in
//!   a sentence; a language model's opinion is not the app's, and the screen says
//!   whose it is.
//!
//! # One OpenAI-compatible shape, local or cloud
//!
//! Both are the same request: `POST <endpoint>/chat/completions` with
//! `{ model, messages, temperature }` and an optional `Authorization: Bearer`. A local
//! server (Ollama, LM Studio, llama.cpp's server, vLLM) answers it at
//! `http://127.0.0.1:…/v1`; a cloud provider answers it at `https://…/v1`. That is the
//! whole of the "local or cloud" distinction here, and it is the learner's to make by
//! typing an address.
//!
//! # The answer is parsed leniently, and never trusted
//!
//! The request asks for a small JSON object, and small local models do not always
//! produce one. So [`parse_review`] takes what it can — a JSON object found anywhere
//! in the reply, otherwise the prose itself — and a reply it cannot structure is shown
//! as it came rather than as an error. There is deliberately no `response_format`
//! field in the request: several local servers reject it, and the app has nothing to
//! lose by reading the answer generously.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How long a request may take to connect, and to answer.
///
/// Ten seconds to connect, two minutes to read. The read timeout is the long one on
/// purpose: a local model on a laptop can spend a while before its first token, and a
/// request the learner can see is running is better than one that gives up on a
/// machine that was about to answer. Nothing waits forever — the learner can leave the
/// screen, and the request dies with the process.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// What the learner configured, if anything.
///
/// `key` is an `Option` because a local server usually wants no key and an empty
/// string is not the same thing as "none": the request omits the header entirely when
/// there is none, and sends `Bearer ` for nothing useful otherwise.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelSettings {
    /// The OpenAI-compatible base, e.g. `http://127.0.0.1:11434/v1` or
    /// `https://api.openai.com/v1`. Stored as typed; [`ModelSettings::chat_url`]
    /// builds the endpoint from it.
    pub endpoint: String,
    /// The model name the server knows it by, e.g. `llama3.1` or `gpt-4o-mini`.
    pub model: String,
    /// The API key, when the endpoint needs one. **This is a secret**, which is why
    /// its file is written readable only by its owner and why it never crosses the IPC
    /// boundary back to the webview.
    pub key: Option<String>,
}

impl ModelSettings {
    /// Whether there is enough here to ask anybody anything.
    ///
    /// A key is not required: most local servers take none.
    pub fn configured(&self) -> bool {
        !self.endpoint.trim().is_empty() && !self.model.trim().is_empty()
    }

    /// The chat-completions URL this endpoint means.
    ///
    /// The learner types the base they were given — `https://api.openai.com/v1`, or
    /// `http://127.0.0.1:11434/v1` — and the path is this app's to know. Trailing
    /// slashes are dropped, and an address that already names the endpoint is left
    /// alone, so both spellings of the same thing work.
    pub fn chat_url(&self) -> String {
        let base = self.endpoint.trim().trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else {
            format!("{base}/chat/completions")
        }
    }

    /// What to call the endpoint on screen: the scheme and host, without the path or
    /// anything else the learner typed.
    ///
    /// It exists so the attribution line can name where a sentence went without
    /// reprinting an address the learner already knows — and so a paragraph of prose
    /// pasted into the field by mistake is not echoed back at them.
    pub fn host(&self) -> String {
        let trimmed = self.endpoint.trim();
        let after_scheme = trimmed.split_once("://").map(|(_, rest)| rest).unwrap_or(trimmed);
        let host = after_scheme.split('/').next().unwrap_or(after_scheme);
        if host.is_empty() {
            trimmed.to_string()
        } else {
            host.to_string()
        }
    }

    /// Whether this is an address the app will send anything to.
    ///
    /// Deliberately only `http` and `https`: a scheme the client cannot speak is a
    /// typo the learner should hear about when they save, rather than a failure on the
    /// first sentence they ask about.
    pub fn validate(&self) -> Result<(), String> {
        let endpoint = self.endpoint.trim();
        if endpoint.is_empty() {
            return Err("the endpoint is empty".to_string());
        }
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return Err(format!(
                "{endpoint:?} is not an address this app can reach: it has to start with http:// or https://"
            ));
        }
        if endpoint.chars().any(char::is_whitespace) {
            return Err("the endpoint has a space in it".to_string());
        }
        if self.model.trim().is_empty() {
            return Err("the model name is empty".to_string());
        }
        Ok(())
    }
}

/// What a model said about one sentence, parsed as far as it can be.
///
/// `verdict` is the model's own word for its judgement, echoed rather than mapped onto
/// anything of this app's. Empty means the reply was not the JSON the prompt asked
/// for, and then `explanation` holds the reply itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelReview {
    /// The model's own verdict word, when it gave one.
    pub verdict: String,
    /// The sentence as the model would write it, when it offered a version.
    pub corrected: Option<String>,
    /// The model's explanation.
    pub explanation: String,
    /// The reply exactly as it arrived, for a screen that wants to show the whole of
    /// it — and for a person who wants to see what the model really said.
    pub raw: String,
}

/// A model's answer about a sentence, and whose answer it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVerdict {
    /// The model's own answer.
    pub review: ModelReview,
    /// The model name the request asked for, echoed so the screen can attribute it.
    pub model: String,
    /// The endpoint's host, for the same reason — never the key, never the full path.
    pub host: String,
    /// What was sent, as one line the learner can read: the pattern and their sentence.
    pub sent: String,
}

/// The system prompt: what the model is being asked for, and in what shape.
///
/// Written out as a constant rather than inline so a test can hold its promises: it
/// asks for JSON, it asks for a beginner-appropriate explanation, and it asks the model
/// to say when a sentence is fine — a checker that only ever finds faults teaches a
/// learner to distrust their own correct sentences.
pub const SYSTEM_PROMPT: &str = "You are a careful Japanese language tutor. A beginner has written one \
sentence to practise a grammar pattern. Judge whether the sentence is grammatical, natural and uses \
the pattern correctly, and whether it means something a Japanese speaker would understand. Be \
specific and encouraging. Reply with a single JSON object and nothing else, in exactly this shape: \
{\"verdict\": \"correct\" | \"understandable\" | \"incorrect\", \"corrected\": \"your corrected \
sentence, or the original if it is right\", \"explanation\": \"one or two short sentences in \
English\"}. If the sentence is correct, say so plainly and do not invent a fault.";

/// The user message: the pattern and the sentence, and nothing of the corpus's prose.
pub fn user_prompt(pattern: &str, reading: Option<&str>, sentence: &str) -> String {
    let mut out = format!("Pattern: {pattern}");
    if let Some(reading) = reading {
        out.push_str(&format!("\nPattern in kana: {reading}"));
    }
    out.push_str("\n\nSentence: ");
    out.push_str(sentence);
    out
}

/// The request body: one OpenAI-compatible chat completion.
///
/// `temperature` is low because this is a check rather than a conversation, and there
/// is no `response_format`: a local server is free to reject that field, and
/// [`parse_review`] reads a plain reply anyway.
pub fn review_body(model: &str, pattern: &str, reading: Option<&str>, sentence: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user_prompt(pattern, reading, sentence) },
        ],
        "temperature": 0.2,
    })
}

/// What the learner is about to send, in one line, for the screen to show before they
/// send it.
pub fn sent_line(pattern: &str, sentence: &str) -> String {
    format!("{pattern} · {sentence}")
}

/// Read a model's reply as far as it can be read.
///
/// Three shapes arrive in practice, and all three are handled rather than two:
///
/// * the JSON object the prompt asked for, alone in the reply;
/// * the same object inside a fenced block or wrapped in prose — small models do this
///   constantly, so the first `{` and the last `}` are what is parsed;
/// * no JSON at all, which is prose and is kept as the explanation.
///
/// A reply with a JSON object whose fields are all unknown is prose too: the fields
/// are looked up under the names the prompt asked for and a few near misses
/// (`judgement`, `correction`, `comment`), because a model that answers the question in
/// the wrong shape has still answered it.
pub fn parse_review(content: &str) -> ModelReview {
    let raw = content.trim().to_string();
    let object = raw
        .find('{')
        .and_then(|start| raw.rfind('}').map(|end| (start, end)))
        .filter(|(start, end)| end > start)
        .and_then(|(start, end)| serde_json::from_str::<Value>(&raw[start..=end]).ok());
    let Some(object) = object else {
        return ModelReview {
            verdict: String::new(),
            corrected: None,
            explanation: raw.clone(),
            raw,
        };
    };
    let text = |names: &[&str]| -> Option<String> {
        names
            .iter()
            .find_map(|name| object.get(name).and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let verdict = text(&["verdict", "judgement", "judgment", "assessment"]).unwrap_or_default();
    let corrected = text(&["corrected", "correction", "correct"]);
    let explanation = text(&["explanation", "comment", "notes", "reason"]).unwrap_or_default();
    if verdict.is_empty() && corrected.is_none() && explanation.is_empty() {
        return ModelReview {
            verdict: String::new(),
            corrected: None,
            explanation: raw.clone(),
            raw,
        };
    }
    ModelReview {
        verdict,
        corrected,
        explanation,
        raw,
    }
}

/// What an endpoint's error body says, for a message a person can act on.
///
/// OpenAI and everything compatible with it answer `{"error": {"message": …}}`, and
/// that message is the difference between "401" and "Incorrect API key provided". A
/// body that is not JSON — a proxy's HTML page, a server that answered nothing — is
/// shown truncated, because nobody needs a whole error page in a status line.
pub fn parse_error(status: u16, body: &str) -> String {
    let trimmed = body.trim();
    if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
        if let Some(message) = json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .or_else(|| json.get("message").and_then(Value::as_str))
        {
            return format!("the endpoint answered {status}: {}", message.trim());
        }
    }
    if trimmed.is_empty() {
        return format!("the endpoint answered {status} with no explanation");
    }
    let short: String = trimmed.chars().take(300).collect();
    format!("the endpoint answered {status}: {short}")
}

/// The one thing this module has to touch a socket through.
///
/// A trait rather than a function for the reason `hanzi-sync`'s `Http` is one: every
/// decision above — what is sent, how the reply is read, what is shown — can then be
/// tested against a fake, and [`UreqHttp`] is left with no decisions in it at all.
pub trait Chat: Send + Sync {
    /// POST a JSON body, and answer the body of a 2xx reply.
    fn post_json(&self, url: &str, bearer: Option<&str>, body: &str) -> Result<String, String>;
}

/// The real client: `ureq` over `rustls`, with the root certificates bundled.
///
/// The same client `crates/hanzi-sync` uses for Dropbox, and chosen for the same
/// reason: it is blocking and small, and `webpki-roots` means no system certificate
/// store is read — which matters under a sandbox and on the mobile builds alike.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(CONNECT_TIMEOUT)
                .timeout_read(READ_TIMEOUT)
                .user_agent(concat!("NihongoTutor/", env!("CARGO_PKG_VERSION")))
                .build(),
        }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl Chat for UreqHttp {
    fn post_json(&self, url: &str, bearer: Option<&str>, body: &str) -> Result<String, String> {
        let mut request = self.agent.post(url).set("Content-Type", "application/json");
        if let Some(key) = bearer.map(str::trim).filter(|key| !key.is_empty()) {
            request = request.set("Authorization", &format!("Bearer {key}"));
        }
        match request.send_string(body) {
            Ok(response) => response
                .into_string()
                .map_err(|err| format!("{url} answered but the body could not be read: {err}")),
            Err(ureq::Error::Status(status, response)) => {
                Err(parse_error(status, &response.into_string().unwrap_or_default()))
            }
            Err(ureq::Error::Transport(transport)) => Err(format!("{url}: {transport}")),
        }
    }
}

/// Ask `chat` to check one sentence against one pattern.
///
/// This is the whole of the feature's decision-making: validate the settings, build
/// the body from the pattern and the sentence **and nothing else**, send it, and read
/// the answer. Every failure is a sentence for the learner — the endpoint could not be
/// reached, the endpoint refused the key, the reply had no text in it — rather than a
/// silent nothing.
pub fn check(
    settings: &ModelSettings,
    pattern: &str,
    reading: Option<&str>,
    sentence: &str,
    chat: &dyn Chat,
) -> Result<ModelReview, String> {
    settings.validate()?;
    let body = review_body(&settings.model, pattern, reading, sentence);
    let serialised = serde_json::to_string(&body).map_err(|err| err.to_string())?;
    let answer = chat.post_json(&settings.chat_url(), settings.key.as_deref(), &serialised)?;
    let content = completion_text(&answer).ok_or_else(|| {
        format!(
            "the endpoint answered with something this app could not read as a chat completion: {}",
            answer.chars().take(300).collect::<String>()
        )
    })?;
    Ok(parse_review(&content))
}

/// The assistant's text out of an OpenAI-compatible reply.
///
/// `choices[0].message.content` is the shape every compatible server uses; the legacy
/// `choices[0].text` is read as well, because it costs one line and some local servers
/// still answer in it.
fn completion_text(answer: &str) -> Option<String> {
    let json: Value = serde_json::from_str(answer.trim()).ok()?;
    if let Some(message) = json
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    {
        return Some(message.to_string());
    }
    json.get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn settings(endpoint: &str) -> ModelSettings {
        ModelSettings {
            endpoint: endpoint.to_string(),
            model: "small-model".to_string(),
            key: None,
        }
    }

    #[test]
    fn the_endpoint_is_the_base_and_the_path_is_this_apps() {
        assert_eq!(
            settings("https://api.openai.com/v1").chat_url(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            settings("https://api.openai.com/v1/").chat_url(),
            "https://api.openai.com/v1/chat/completions",
            "a trailing slash is the same address"
        );
        assert_eq!(
            settings("http://127.0.0.1:11434/v1/chat/completions").chat_url(),
            "http://127.0.0.1:11434/v1/chat/completions",
            "and an address that already names it is left alone"
        );
        assert_eq!(
            settings(" http://127.0.0.1:1234/v1 ").chat_url(),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
    }

    #[test]
    fn the_host_is_what_an_attribution_line_needs_and_nothing_more() {
        assert_eq!(settings("https://api.openai.com/v1").host(), "api.openai.com");
        assert_eq!(settings("http://127.0.0.1:11434/v1").host(), "127.0.0.1:11434");
        assert_eq!(settings("not an address at all").host(), "not an address at all");
        assert_eq!(settings("").host(), "");
    }

    #[test]
    fn a_key_is_optional_because_a_local_server_takes_none() {
        assert!(!ModelSettings::default().configured());
        assert!(settings("http://127.0.0.1:1234/v1").configured(), "no key, still configured");
        assert!(
            !ModelSettings {
                endpoint: "http://127.0.0.1:1234/v1".to_string(),
                model: "  ".to_string(),
                key: None
            }
            .configured(),
            "a model name is not optional: the request has to name one"
        );
    }

    #[test]
    fn an_address_a_client_cannot_speak_is_refused_when_it_is_saved() {
        assert!(settings("http://127.0.0.1:1234/v1").validate().is_ok());
        assert!(settings("https://api.openai.com/v1").validate().is_ok());
        for bad in ["", "  ", "127.0.0.1:1234", "ftp://example.com", "http://two words/v1"] {
            assert!(settings(bad).validate().is_err(), "{bad:?} should be refused");
        }
        assert!(
            ModelSettings {
                endpoint: "http://127.0.0.1:1234/v1".to_string(),
                model: String::new(),
                key: None
            }
            .validate()
            .is_err(),
            "and so is a request that names no model"
        );
    }

    #[test]
    fn the_request_carries_the_pattern_and_the_sentence_and_no_corpus_prose() {
        let body = review_body("small-model", "〜の方が〜より", Some("〜のほうが〜より"), "いぬのほうがねこより。");
        assert_eq!(body["model"], "small-model");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], SYSTEM_PROMPT);
        assert_eq!(body["messages"][1]["role"], "user");
        let sent = body["messages"][1]["content"].as_str().expect("a string");
        assert!(sent.contains("〜の方が〜より"), "{sent}");
        assert!(sent.contains("〜のほうが〜より"), "{sent}");
        assert!(sent.contains("いぬのほうがねこより。"), "{sent}");
        assert_eq!(body["temperature"], 0.2);
        // The corpus's prose is not in the request: only the pattern and the sentence.
        // "Noun + の方が + Noun + より" and the note are OpenJLPT's text, and this app
        // does not hand a corpus to a third party to check one sentence.
        assert_eq!(body.as_object().expect("an object").len(), 3, "model, messages, temperature");
    }

    #[test]
    fn a_reply_in_the_shape_the_prompt_asked_for_is_read_field_by_field() {
        let review = parse_review(
            r#"{"verdict": "understandable", "corrected": "犬の方が猫より好きです。", "explanation": "The pattern is right; add the topic."}"#,
        );
        assert_eq!(review.verdict, "understandable");
        assert_eq!(review.corrected.as_deref(), Some("犬の方が猫より好きです。"));
        assert_eq!(review.explanation, "The pattern is right; add the topic.");
        assert_eq!(review.raw, r#"{"verdict": "understandable", "corrected": "犬の方が猫より好きです。", "explanation": "The pattern is right; add the topic."}"#);
    }

    #[test]
    fn the_same_object_inside_a_fence_or_a_paragraph_is_still_read() {
        let fenced = parse_review(
            "Sure! Here it is:\n```json\n{\"verdict\": \"correct\", \"corrected\": \"この本は面白いです。\", \"explanation\": \"Good.\"}\n```\nHope that helps.",
        );
        assert_eq!(fenced.verdict, "correct");
        assert_eq!(fenced.corrected.as_deref(), Some("この本は面白いです。"));
        assert_eq!(fenced.explanation, "Good.");
    }

    #[test]
    fn a_reply_with_no_json_is_prose_and_is_kept_as_it_came() {
        let review = parse_review("Your sentence is fine, though より is usually optional here.");
        assert!(review.verdict.is_empty(), "no verdict word was given, so none is invented");
        assert_eq!(review.corrected, None);
        assert!(review.explanation.contains("usually optional"));
        assert_eq!(review.raw, review.explanation);

        // A JSON object whose fields are all unknown is prose too: the model answered
        // in a shape this app does not know, and its answer is still worth showing.
        let odd = parse_review(r#"{"answer": "looks fine", "score": 9}"#);
        assert!(odd.verdict.is_empty());
        assert!(odd.explanation.contains("looks fine"));
    }

    #[test]
    fn an_empty_reply_is_an_empty_review_rather_than_a_panic() {
        let review = parse_review("   ");
        assert_eq!(review.verdict, "");
        assert_eq!(review.corrected, None);
        assert_eq!(review.explanation, "");
        assert_eq!(review.raw, "");
    }

    /// A `Chat` that answers from memory, and remembers what it was asked.
    ///
    /// The same seam `hanzi-sync`'s in-memory Dropbox uses: every decision above is
    /// tested without a socket, and [`UreqHttp`] is left with none to test.
    struct FakeChat {
        answer: Result<String, String>,
        seen: std::sync::Mutex<Vec<(String, Option<String>, String)>>,
    }

    impl Default for FakeChat {
        fn default() -> Self {
            Self {
                answer: Ok(String::new()),
                seen: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl Chat for FakeChat {
        fn post_json(&self, url: &str, bearer: Option<&str>, body: &str) -> Result<String, String> {
            self.seen
                .lock()
                .expect("the fake's log is not poisoned")
                .push((url.to_string(), bearer.map(str::to_string), body.to_string()));
            self.answer.clone()
        }
    }

    fn reply(content: &str) -> String {
        json!({ "choices": [{ "message": { "role": "assistant", "content": content } }] })
            .to_string()
    }

    #[test]
    fn a_check_sends_the_learners_own_two_things_and_returns_the_models_answer() {
        let chat = FakeChat {
            answer: Ok(reply(r#"{"verdict":"correct","corrected":"いぬのほうがねこよりすきです。","explanation":"Right."}"#)),
            ..FakeChat::default()
        };
        let mut settings = settings("http://127.0.0.1:1234/v1");
        settings.key = Some("sk-local".to_string());
        let review = check(
            &settings,
            "〜の方が〜より",
            Some("〜のほうが〜より"),
            "いぬのほうがねこよりすきです。",
            &chat,
        )
        .expect("a review");
        assert_eq!(review.verdict, "correct");
        assert_eq!(review.explanation, "Right.");

        let seen = chat.seen.lock().expect("the fake's log");
        assert_eq!(seen.len(), 1);
        let (url, bearer, body) = &seen[0];
        assert_eq!(url, "http://127.0.0.1:1234/v1/chat/completions");
        assert_eq!(bearer.as_deref(), Some("sk-local"));
        assert!(body.contains("いぬのほうがねこよりすきです。"));
        assert!(body.contains("small-model"));
    }

    #[test]
    fn a_key_that_is_unset_sends_no_authorization_header_at_all() {
        let chat = FakeChat {
            answer: Ok(reply("fine")),
            ..FakeChat::default()
        };
        let settings = settings("http://127.0.0.1:1234/v1");
        assert!(settings.key.is_none());
        check(&settings, "〜は", None, "これは本です。", &chat).expect("a review");
        let seen = chat.seen.lock().expect("the fake's log");
        assert_eq!(seen[0].1, None, "an empty key is not a Bearer with nothing after it");
    }

    #[test]
    fn the_four_failures_are_sentences_rather_than_nothings() {
        let settings = settings("http://127.0.0.1:1234/v1");
        // The endpoint refused to answer.
        let refused = FakeChat {
            answer: Err("the endpoint answered 401: Incorrect API key provided".to_string()),
            ..FakeChat::default()
        };
        let err = check(&settings, "〜は", None, "これは本です。", &refused).unwrap_err();
        assert!(err.contains("Incorrect API key provided"), "{err}");

        // The endpoint answered something that is not a chat completion.
        let odd = FakeChat {
            answer: Ok("<html>hello</html>".to_string()),
            ..FakeChat::default()
        };
        let err = check(&settings, "〜は", None, "これは本です。", &odd).unwrap_err();
        assert!(err.contains("could not read as a chat completion"), "{err}");

        // Settings that name no address are refused before anything is sent.
        let nothing = FakeChat {
            answer: Ok(reply("fine")),
            ..FakeChat::default()
        };
        let err = check(&ModelSettings::default(), "〜は", None, "これは本です。", &nothing).unwrap_err();
        assert!(err.contains("empty"), "{err}");
        assert!(
            nothing.seen.lock().expect("the fake's log").is_empty(),
            "and nothing was sent"
        );

        // A reply with no content at all is read as an empty review, not an error.
        let blank = FakeChat {
            answer: Ok(json!({ "choices": [{ "message": { "content": "" } }] }).to_string()),
            ..FakeChat::default()
        };
        let review = check(&settings, "〜は", None, "これは本です。", &blank).expect("a review");
        assert_eq!(review, ModelReview::default());
    }

    /// Serve exactly one HTTP request from a socket on this machine, and hand back the
    /// address to send it to plus the request that arrived.
    ///
    /// The real client is the one part of this module a fake cannot stand in for, and
    /// this is how it is tested without the internet: a listener on `127.0.0.1:0`, one
    /// canned reply, and the bytes `UreqHttp` actually wrote. It is the same idea as
    /// the harness the app's interface probes use, one layer down.
    fn serve_once(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port on this machine");
        let address = listener.local_addr().expect("the address it took");
        // Owned before the thread starts: a `&str` here would tie the server's whole
        // life to the caller's stack frame.
        let status = status.to_string();
        let body = body.to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one connection");
            let request = read_request(&mut stream);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            use std::io::Write;
            stream.write_all(response.as_bytes()).expect("the reply");
            let _ = stream.flush();
            request
        });
        (format!("http://{address}/v1/chat/completions"), handle)
    }

    /// Read one HTTP request: the head, then as much body as it announced.
    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        let mut expected = None;
        loop {
            let read = stream.read(&mut buffer).unwrap_or(0);
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request).to_string();
            if expected.is_none() {
                if let Some(head_end) = text.find("\r\n\r\n") {
                    let length = text[..head_end]
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);
                    expected = Some(head_end + 4 + length);
                }
            }
            if expected.is_some_and(|total| request.len() >= total) {
                break;
            }
        }
        String::from_utf8_lossy(&request).to_string()
    }

    #[test]
    fn the_real_client_posts_the_body_it_built_and_reads_the_answer_back() {
        let (url, server) = serve_once(
            "200 OK",
            &reply(r#"{"verdict":"correct","corrected":"これは本です。","explanation":"Fine."}"#),
        );
        let mut settings = settings("http://127.0.0.1:1/unused");
        // The address comes from the listener, so the settings are pointed at it after
        // the fact rather than the port being guessed.
        settings.endpoint = url.trim_end_matches("/chat/completions").to_string();
        settings.key = Some("sk-test".to_string());
        let review = check(&settings, "〜は", None, "これは本です。", &UreqHttp::new())
            .expect("the local server answered");
        assert_eq!(review.verdict, "correct");
        assert_eq!(review.corrected.as_deref(), Some("これは本です。"));

        let request = server.join().expect("the server thread");
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"), "{request}");
        assert!(request.contains("authorization: Bearer sk-test") || request.contains("Authorization: Bearer sk-test"), "{request}");
        assert!(request.contains("\"model\":\"small-model\""), "{request}");
        assert!(request.contains("これは本です。"), "{request}");
    }

    #[test]
    fn a_refused_key_reaches_the_learner_as_what_the_endpoint_said() {
        let (url, server) = serve_once(
            "401 Unauthorized",
            r#"{"error": {"message": "Incorrect API key provided", "type": "invalid_request_error"}}"#,
        );
        let mut settings = settings("http://127.0.0.1:1/unused");
        settings.endpoint = url.trim_end_matches("/chat/completions").to_string();
        let error = check(&settings, "〜は", None, "これは本です。", &UreqHttp::new()).unwrap_err();
        assert_eq!(error, "the endpoint answered 401: Incorrect API key provided");
        let _ = server.join();
    }

    #[test]
    fn nothing_listening_is_a_message_rather_than_a_hang() {
        // A port that was just closed: the connection is refused immediately, which is
        // what a learner who has not started their local server will meet.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listener.local_addr().expect("the address");
        drop(listener);
        let mut settings = settings("http://127.0.0.1:1/unused");
        settings.endpoint = format!("http://{address}/v1");
        let error = check(&settings, "〜は", None, "これは本です。", &UreqHttp::new()).unwrap_err();
        assert!(error.contains(&format!("http://{address}/v1/chat/completions")), "{error}");
    }

    #[test]
    fn an_error_body_becomes_the_sentence_the_endpoint_wrote() {
        assert_eq!(
            parse_error(401, r#"{"error": {"message": "Incorrect API key provided", "type": "invalid_request_error"}}"#),
            "the endpoint answered 401: Incorrect API key provided"
        );
        assert_eq!(
            parse_error(500, r#"{"message": "model not found"}"#),
            "the endpoint answered 500: model not found"
        );
        assert_eq!(parse_error(502, ""), "the endpoint answered 502 with no explanation");
        let html = parse_error(404, "<html><body>Not Found</body></html>");
        assert!(html.starts_with("the endpoint answered 404: <html>"), "{html}");
    }
}
