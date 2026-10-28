//! Plain-language edits the phrase reader could not follow ("cut the part
//! where I talk about pricing"), turned into edits by a model the user
//! chooses. Only the instruction, the duration and the transcript's text with
//! its times are sent; the media never leaves the machine.

use anyhow::{Context, Result, bail};
use serde_json::json;
use verbatim_edl::{PLAN_SYSTEM, Plan, Transcript, parse_plan, plan_prompt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Provider {
    Anthropic,
    Openai,
    Ollama,
}

pub struct Ai {
    pub provider: Provider,
    pub model: String,
    key: Option<String>,
    base: String,
}

impl Ai {
    /// `--ai`, or `VERBATIM_AI`, or the first API key found in the environment.
    pub fn from_env(provider: Option<Provider>, model: Option<String>) -> Result<Self> {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let provider = match provider {
            Some(p) => p,
            None => match env("VERBATIM_AI").as_deref() {
                Some("anthropic") => Provider::Anthropic,
                Some("openai") => Provider::Openai,
                Some("ollama") => Provider::Ollama,
                Some(other) => bail!("VERBATIM_AI={other}: use anthropic, openai or ollama"),
                None if env("ANTHROPIC_API_KEY").is_some() => Provider::Anthropic,
                None if env("OPENAI_API_KEY").is_some() => Provider::Openai,
                None => bail!(
                    "this instruction needs a model: set ANTHROPIC_API_KEY or OPENAI_API_KEY, or run Ollama and pass --ai ollama --ai-model <model>. Times alone (\"de 15:39 à 15:43\", \"the last 5:42\") need no model"
                ),
            },
        };
        let model = model.or_else(|| env("VERBATIM_AI_MODEL"));
        Ok(match provider {
            Provider::Anthropic => Self {
                provider,
                model: model.unwrap_or_else(|| "claude-sonnet-5-5".into()),
                key: Some(env("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY is not set")?),
                base: env("ANTHROPIC_BASE_URL")
                    .unwrap_or_else(|| "https://api.anthropic.com".into()),
            },
            Provider::Openai => Self {
                provider,
                model: model.context(
                    "pass --ai-model (or VERBATIM_AI_MODEL) for an OpenAI-compatible API",
                )?,
                key: Some(env("OPENAI_API_KEY").context("OPENAI_API_KEY is not set")?),
                base: env("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com/v1".into()),
            },
            Provider::Ollama => Self {
                provider,
                model: model.context(
                    "pass --ai-model (or VERBATIM_AI_MODEL), for instance --ai-model llama3.2",
                )?,
                key: None,
                base: env("OLLAMA_BASE_URL").unwrap_or_else(|| "http://localhost:11434/v1".into()),
            },
        })
    }

    fn complete(&self, system: &str, user: &str) -> Result<String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()?;
        let res = match self.provider {
            Provider::Anthropic => client
                .post(format!("{}/v1/messages", self.base.trim_end_matches('/')))
                .header("x-api-key", self.key.as_deref().unwrap_or_default())
                .header("anthropic-version", "2023-06-01")
                .json(&json!({ "model": self.model, "max_tokens": 4096, "system": system, "messages": [{ "role": "user", "content": user }] })),
            Provider::Openai | Provider::Ollama => {
                let mut req = client.post(format!("{}/chat/completions", self.base.trim_end_matches('/'))).json(&json!({
                    "model": self.model,
                    "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }],
                }));
                if let Some(k) = &self.key {
                    req = req.bearer_auth(k);
                }
                req
            }
        }
        .send()
        .with_context(|| format!("calling {}", self.base))?;
        let status = res.status();
        let body: serde_json::Value = res.json().context("the model's answer is not JSON")?;
        if !status.is_success() {
            let msg = body["error"]["message"]
                .as_str()
                .map_or_else(|| body.to_string(), str::to_string);
            bail!("{:?} answered {status}: {msg}", self.provider);
        }
        let text = match self.provider {
            Provider::Anthropic => body["content"].as_array().map(|c| {
                c.iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<String>()
            }),
            _ => body["choices"][0]["message"]["content"]
                .as_str()
                .map(str::to_string),
        };
        text.filter(|t| !t.trim().is_empty())
            .context("the model gave an empty answer")
    }
}

pub fn plan(
    ai: &Ai,
    instruction: &str,
    duration: f64,
    transcript: Option<&Transcript>,
) -> Result<Plan> {
    let text = ai.complete(PLAN_SYSTEM, &plan_prompt(instruction, duration, transcript))?;
    parse_plan(&text).map_err(anyhow::Error::msg)
}
