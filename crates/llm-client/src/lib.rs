use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::Mutex,
    time::timeout,
};

#[derive(Debug)]
struct RetryableUsageError(String);

impl std::fmt::Display for RetryableUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RetryableUsageError {}

struct UsageReading {
    completed: Instant,
    remaining: Result<f64, String>,
}

type UsageSlot = Arc<Mutex<Option<UsageReading>>>;
static USAGE_CHECKS: LazyLock<Mutex<HashMap<String, UsageSlot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub struct CodexClient {
    pub executable: String,
    pub model: String,
    pub reasoning_effort: String,
    pub timeout: Duration,
    pub web_search: bool,
}

impl CodexClient {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.model.trim().is_empty(), "model is required");
        ensure!(
            matches!(
                self.reasoning_effort.as_str(),
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
            ),
            "explicit, valid reasoning effort is required"
        );
        Ok(())
    }

    pub async fn check_remaining_usage(&self, minimum: f64) -> Result<()> {
        ensure!(
            (0.0..=100.0).contains(&minimum),
            "usage threshold must be between 0 and 100"
        );
        let requested = Instant::now();
        let slot = USAGE_CHECKS
            .lock()
            .await
            .entry(self.executable.clone())
            .or_default()
            .clone();
        let mut reading = slot.lock().await;
        if !reading.as_ref().is_some_and(|r| {
            r.completed >= requested
                || (r.remaining.is_ok() && r.completed.elapsed() < Duration::from_secs(1))
        }) {
            let remaining = self
                .read_remaining_usage()
                .await
                .map_err(|error| format!("{error:#}"));
            *reading = Some(UsageReading {
                completed: Instant::now(),
                remaining,
            });
        }
        let remaining = reading
            .as_ref()
            .unwrap()
            .remaining
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        ensure!(
            *remaining >= minimum,
            "Codex remaining usage {remaining:.1}% is below {minimum:.1}% threshold"
        );
        Ok(())
    }

    async fn read_remaining_usage(&self) -> Result<f64> {
        let mut attempt = 0;
        let result = loop {
            let error = match self.request_usage().await {
                Ok(result) => break result,
                Err(error) => error,
            };
            if attempt == 2
                || !error.is::<RetryableUsageError>()
                    && !error.is::<tokio::time::error::Elapsed>()
                    && !error.downcast_ref::<std::io::Error>().is_some_and(|e| {
                        matches!(
                            e.kind(),
                            std::io::ErrorKind::BrokenPipe
                                | std::io::ErrorKind::UnexpectedEof
                                | std::io::ErrorKind::ConnectionReset
                        )
                    })
            {
                return Err(error);
            }
            log::warn!("Codex usage check failed, retrying: {error:#}");
            tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
            attempt += 1;
        };
        ensure!(
            result["ordinaryUsageAllowed"] != false,
            "Codex included usage is unavailable"
        );
        let mut remaining: Option<f64> = None;
        for bucket in std::iter::once(&result["rateLimits"]).chain(
            result["rateLimitsByLimitId"]
                .as_object()
                .into_iter()
                .flat_map(|buckets| buckets.values()),
        ) {
            ensure!(
                bucket["spendControlReached"] != true,
                "Codex spending limit reached"
            );
            for window in ["primary", "secondary"] {
                if bucket[window].is_null() {
                    continue;
                }
                let used = bucket[window]["usedPercent"]
                    .as_f64()
                    .context("Codex usage percentage unavailable")?;
                ensure!(
                    (0.0..=100.0).contains(&used),
                    "invalid Codex usage percentage"
                );
                remaining =
                    Some(remaining.map_or(100.0 - used, |previous| previous.min(100.0 - used)));
            }
        }
        remaining.context("Codex remaining usage unavailable")
    }

    async fn request_usage(&self) -> Result<Value> {
        let mut child = Command::new(&self.executable)
            .args(["--no-daemon", "app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_remove("FRED_API_KEY")
            .env_remove("ALPACA_KEY")
            .env_remove("ALPACA_SECRET")
            .kill_on_drop(true)
            .spawn()
            .context("could not start Codex usage check")?;
        let mut phase = "initialization";
        let result = timeout(Duration::from_secs(30), async {
            let mut stdin = child.stdin.take().context("usage check stdin unavailable")?;
            let mut lines = BufReader::new(child.stdout.take().context("usage check stdout unavailable")?).lines();
            stdin.write_all(b"{\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"stock_pipeline\",\"version\":\"0.1\"},\"capabilities\":null}}\n").await?;
            while let Some(line) = lines.next_line().await? {
                let message: Value = serde_json::from_str(&line)?;
                if let Some(error) = message.get("error").filter(|e| !e.is_null())
                    && (message["id"] == 1 || message["id"] == 2)
                {
                    let code = error["code"].as_i64();
                    let detail = format!("Codex {phase} failed ({}): {}", error["code"], error["message"].as_str().unwrap_or("unknown error"));
                    if matches!(code, Some(-32603 | -32099..=-32000 | 429 | 500..=599)) {
                        return Err(RetryableUsageError(detail).into());
                    }
                    anyhow::bail!(detail);
                }
                if message["id"] == 1 {
                    phase = "account/rateLimits/read";
                    stdin.write_all(b"{\"method\":\"initialized\"}\n{\"id\":2,\"method\":\"account/rateLimits/read\"}\n").await?;
                } else if message["id"] == 2 {
                    return message.get("result").cloned().context("Codex usage response missing result");
                }
            }
            Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, format!("Codex usage check ended during {phase}")).into())
        }).await.with_context(|| format!("Codex usage check timed out during {phase}"));
        child.kill().await.ok();
        child.wait().await.ok();
        result?
    }

    pub async fn execute<T: DeserializeOwned>(&self, prompt: &str, schema: &Value) -> Result<T> {
        self.execute_with_directory(prompt, schema, None, true)
            .await
    }

    pub async fn execute_in_directory<T: DeserializeOwned>(
        &self,
        prompt: &str,
        schema: &Value,
        directory: &Path,
    ) -> Result<T> {
        self.execute_with_directory(prompt, schema, Some(directory), false)
            .await
    }

    pub async fn execute_with_document<T: DeserializeOwned>(
        &self,
        prompt: &str,
        schema: &Value,
        document: &str,
    ) -> Result<T> {
        let directory = tempfile::tempdir()?;
        std::fs::write(directory.path().join("prospectus.txt"), document)?;
        self.execute_with_directory(prompt, schema, Some(directory.path()), true)
            .await
    }

    async fn execute_with_directory<T: DeserializeOwned>(
        &self,
        prompt: &str,
        schema: &Value,
        working_directory: Option<&Path>,
        read_only: bool,
    ) -> Result<T> {
        self.validate()?;
        let directory = tempfile::tempdir()?;
        std::fs::write(
            directory.path().join("response-schema.json"),
            serde_json::to_vec(schema)?,
        )?;
        let mut command = Command::new(&self.executable);
        command
            .args([
                "--no-daemon",
                "--ask-for-approval",
                "never",
                "exec",
                "--ignore-user-config",
                "--config",
                "project_doc_max_bytes=0",
                "--sandbox",
                if read_only {
                    "read-only"
                } else {
                    "danger-full-access"
                },
                "--skip-git-repo-check",
                "--ephemeral",
                "--color",
                "never",
            ])
            .arg("--output-schema")
            .arg(directory.path().join("response-schema.json"))
            .arg("--output-last-message")
            .arg(directory.path().join("response.json"))
            .current_dir(working_directory.unwrap_or(directory.path()))
            .env_remove("FRED_API_KEY")
            .env_remove("ALPACA_KEY")
            .env_remove("ALPACA_SECRET")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if !self.web_search && working_directory.is_none() {
            let catalog = timeout(
                self.timeout,
                Command::new(&self.executable)
                    .args(["debug", "models"])
                    .kill_on_drop(true)
                    .output(),
            )
            .await
            .context("Codex model discovery timed out")??;
            ensure!(catalog.status.success(), "Codex model discovery failed");
            let mut catalog: Value = serde_json::from_slice(&catalog.stdout)?;
            let models = catalog["models"]
                .as_array_mut()
                .context("Codex model catalog is missing models")?;
            models.retain(|model| model["slug"].as_str() == Some(&self.model));
            ensure!(
                !models.is_empty(),
                "model {} not found in Codex catalog",
                self.model
            );
            for model in models {
                model["tool_mode"] = "disabled".into();
                model["multi_agent_version"] = "disabled".into();
                model["experimental_supported_tools"] = serde_json::json!([]);
                model["apply_patch_tool_type"] = Value::Null;
            }
            std::fs::write(
                directory.path().join("models.json"),
                serde_json::to_vec(&catalog)?,
            )?;
            command.args([
                "--config",
                &format!(
                    "model_catalog_json={}",
                    serde_json::to_string(&directory.path().join("models.json"))?
                ),
            ]);
            for feature in [
                "shell_tool",
                "unified_exec",
                "multi_agent",
                "multi_agent_v2",
                "apps",
                "plugins",
                "browser_use",
                "computer_use",
                "image_generation",
                "view_image",
                "code_mode",
                "code_mode_host",
                "skill_search",
                "sleep_tool",
                "goals",
                "hooks",
            ] {
                command.args(["--disable", feature]);
            }
        }
        if !self.web_search && working_directory.is_some() {
            for feature in [
                "apps",
                "plugins",
                "multi_agent",
                "multi_agent_v2",
                "browser_use",
                "computer_use",
                "image_generation",
            ] {
                command.args(["--disable", feature]);
            }
        }
        command.args([
            "--config",
            "tools.update_plan.enabled=false",
            "--config",
            "tools.experimental_request_user_input.enabled=false",
            "--config",
            if self.web_search {
                "web_search=\"live\""
            } else {
                "web_search=\"disabled\""
            },
        ]);
        command.args([
            "--model",
            &self.model,
            "--config",
            &format!(
                "model_reasoning_effort={}",
                serde_json::to_string(&self.reasoning_effort)?
            ),
        ]);
        let mut child = command.arg("-").spawn().context("could not start Codex")?;
        let call = child.id().context("Codex process ID unavailable")?;
        log::debug!(
            "Codex {call} · model={} · web={} · prompt:\n{prompt}",
            self.model,
            self.web_search
        );
        let mut stdin = child.stdin.take().context("Codex stdin unavailable")?;
        let mut stderr =
            BufReader::new(child.stderr.take().context("Codex stderr unavailable")?).lines();
        let result = timeout(self.timeout, async {
            let (status, ()) = tokio::try_join!(
                async {
                    stdin.write_all(prompt.as_bytes()).await?;
                    stdin.shutdown().await?;
                    drop(stdin);
                    child.wait().await
                },
                async {
                    while let Some(line) = stderr.next_line().await? {
                        log::debug!("Codex {call}: {line}");
                    }
                    Ok::<(), std::io::Error>(())
                }
            )?;
            ensure!(status.success(), "Codex exited with {status}");
            let response = std::fs::read(directory.path().join("response.json"))?;
            log::debug!(
                "Codex {call} response: {}",
                String::from_utf8_lossy(&response)
            );
            serde_json::from_slice(&response).context("invalid Codex response")
        })
        .await;
        match result {
            Ok(result) => result,
            Err(_) => {
                child.kill().await.ok();
                child.wait().await.ok();
                anyhow::bail!("Codex call timed out")
            }
        }
    }
}
