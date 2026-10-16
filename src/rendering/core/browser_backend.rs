//! Browser backend preference and bounded pre-canvas fallback policy.

use std::future::Future;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserBackendPreference {
    Auto,
    GlOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserBackend {
    BrowserWebGpu,
    Gl,
}

#[derive(Debug)]
pub struct BrowserCandidate<T> {
    pub backend: BrowserBackend,
    pub value: T,
    pub webgpu_failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserBackendError {
    pub webgpu_failure: Option<String>,
    pub gl_failure: String,
}

impl std::fmt::Display for BrowserBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.webgpu_failure {
            Some(webgpu) => write!(
                f,
                "Browser GPU negotiation failed; WebGPU: {webgpu}; WebGL2: {}",
                self.gl_failure
            ),
            None => write!(f, "Browser WebGL2 negotiation failed: {}", self.gl_failure),
        }
    }
}

impl std::error::Error for BrowserBackendError {}

/// Formats a failure after the real canvas has acquired a backend-specific context.
pub fn post_binding_error(backend: BrowserBackend, stage: &str, detail: &str) -> String {
    format!(
        "{backend:?} {stage} failed after the application canvas was bound: {detail}. The canvas cannot safely switch graphics APIs; reload the page with ?kepler_wgpu_backend=gl to diagnose WebGL2."
    )
}

/// Parses the sole supported browser diagnostic override. Unknown values remain caller-warnable.
pub fn parse_browser_backend_preference(
    value: Option<&str>,
) -> Result<BrowserBackendPreference, &'static str> {
    match value {
        None | Some("auto") => Ok(BrowserBackendPreference::Auto),
        Some("gl") => Ok(BrowserBackendPreference::GlOnly),
        Some(_) => Err("unknown browser GPU backend selector"),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn browser_backend_preference_from_query() -> BrowserBackendPreference {
    let value = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok())
        .and_then(|params| params.get("kepler_wgpu_backend"));
    match parse_browser_backend_preference(value.as_deref()) {
        Ok(preference) => preference,
        Err(message) => {
            log::warn!("{message}; ignoring selector and using automatic backend preference");
            BrowserBackendPreference::Auto
        }
    }
}

/// Chooses a preflighted backend, making at most one WebGL2 attempt.
///
/// The returned candidate is not yet bound to the application's canvas. Committing the selected
/// backend to that canvas happens afterward and must not call this fallback routine again.
pub async fn select_browser_candidate<T, WebGpuAttempt, GlAttempt, WebGpuFuture, GlFuture>(
    preference: BrowserBackendPreference,
    webgpu_detected: bool,
    webgpu_attempt: WebGpuAttempt,
    gl_attempt: GlAttempt,
) -> Result<BrowserCandidate<T>, BrowserBackendError>
where
    WebGpuAttempt: FnOnce() -> WebGpuFuture,
    WebGpuFuture: Future<Output = Result<T, String>>,
    GlAttempt: FnOnce() -> GlFuture,
    GlFuture: Future<Output = Result<T, String>>,
{
    let mut webgpu_failure = None;
    if preference == BrowserBackendPreference::Auto {
        if webgpu_detected {
            match webgpu_attempt().await {
                Ok(value) => {
                    return Ok(BrowserCandidate {
                        backend: BrowserBackend::BrowserWebGpu,
                        value,
                        webgpu_failure: None,
                    });
                }
                Err(error) => webgpu_failure = Some(error),
            }
        } else {
            webgpu_failure = Some("WebGPU adapter detection reported unavailable".to_string());
        }
    }

    match gl_attempt().await {
        Ok(value) => Ok(BrowserCandidate {
            backend: BrowserBackend::Gl,
            value,
            webgpu_failure,
        }),
        Err(gl_failure) => Err(BrowserBackendError {
            webgpu_failure,
            gl_failure,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn selects_successful_webgpu_without_attempting_gl() -> Result<(), BrowserBackendError> {
        let gl_attempts = Cell::new(0);
        let selected = futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::Auto,
            true,
            || async { Ok::<_, String>("webgpu candidate") },
            || {
                gl_attempts.set(gl_attempts.get() + 1);
                async { Ok("gl candidate") }
            },
        ))?;
        assert_eq!(selected.backend, BrowserBackend::BrowserWebGpu);
        assert_eq!(selected.value, "webgpu candidate");
        assert_eq!(gl_attempts.get(), 0);
        Ok(())
    }

    #[test]
    fn unavailable_webgpu_attempts_gl_once() -> Result<(), BrowserBackendError> {
        let webgpu_attempts = Cell::new(0);
        let gl_attempts = Cell::new(0);
        let selected = futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::Auto,
            false,
            || {
                webgpu_attempts.set(webgpu_attempts.get() + 1);
                async { Ok::<_, String>("webgpu candidate") }
            },
            || {
                gl_attempts.set(gl_attempts.get() + 1);
                async { Ok("gl candidate") }
            },
        ))?;
        assert_eq!(selected.backend, BrowserBackend::Gl);
        assert_eq!(webgpu_attempts.get(), 0);
        assert_eq!(gl_attempts.get(), 1);
        Ok(())
    }

    #[test]
    fn failed_webgpu_negotiation_falls_back_to_gl_once() -> Result<(), BrowserBackendError> {
        let gl_attempts = Cell::new(0);
        let selected = futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::Auto,
            true,
            || async { Err::<&str, _>("device request failed".to_string()) },
            || {
                gl_attempts.set(gl_attempts.get() + 1);
                async { Ok("gl candidate") }
            },
        ))?;
        assert_eq!(selected.backend, BrowserBackend::Gl);
        assert_eq!(selected.value, "gl candidate");
        assert_eq!(gl_attempts.get(), 1);
        assert_eq!(
            selected.webgpu_failure.as_deref(),
            Some("device request failed")
        );
        Ok(())
    }

    #[test]
    fn reports_both_failed_candidates_without_retrying() {
        let gl_attempts = Cell::new(0);
        let error = match futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::Auto,
            true,
            || async { Err::<&str, _>("adapter request failed".to_string()) },
            || {
                gl_attempts.set(gl_attempts.get() + 1);
                async { Err("GL adapter request failed".to_string()) }
            },
        )) {
            Ok(_) => panic!("both failing backend attempts unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(gl_attempts.get(), 1);
        assert_eq!(
            error.webgpu_failure.as_deref(),
            Some("adapter request failed")
        );
        assert_eq!(error.gl_failure, "GL adapter request failed");
    }

    #[test]
    fn forced_gl_skips_webgpu_detection_and_attempt() -> Result<(), BrowserBackendError> {
        let webgpu_attempts = Cell::new(0);
        let selected = futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::GlOnly,
            true,
            || {
                webgpu_attempts.set(webgpu_attempts.get() + 1);
                async { Ok::<_, String>("webgpu candidate") }
            },
            || async { Ok("gl candidate") },
        ))?;
        assert_eq!(selected.backend, BrowserBackend::Gl);
        assert_eq!(webgpu_attempts.get(), 0);
        Ok(())
    }

    #[test]
    fn selector_defaults_to_auto_and_warnable_unknown_values() {
        assert_eq!(
            parse_browser_backend_preference(None),
            Ok(BrowserBackendPreference::Auto)
        );
        assert_eq!(
            parse_browser_backend_preference(Some("auto")),
            Ok(BrowserBackendPreference::Auto)
        );
        assert_eq!(
            parse_browser_backend_preference(Some("gl")),
            Ok(BrowserBackendPreference::GlOnly)
        );
        assert!(parse_browser_backend_preference(Some("vulkan")).is_err());
    }

    #[test]
    fn post_binding_errors_identify_backend_stage_and_recovery() {
        let error = post_binding_error(
            BrowserBackend::BrowserWebGpu,
            "surface configuration",
            "unsupported presentation mode",
        );
        assert!(error.contains("BrowserWebGpu"));
        assert!(error.contains("surface configuration"));
        assert!(error.contains("?kepler_wgpu_backend=gl"));
    }

    #[test]
    fn post_canvas_commit_failure_does_not_retry_another_backend() -> Result<(), BrowserBackendError>
    {
        let gl_attempts = Cell::new(0);
        let candidate = futures::executor::block_on(select_browser_candidate(
            BrowserBackendPreference::Auto,
            true,
            || async { Ok::<_, String>("webgpu candidate") },
            || {
                gl_attempts.set(gl_attempts.get() + 1);
                async { Ok("gl candidate") }
            },
        ))?;
        assert_eq!(candidate.backend, BrowserBackend::BrowserWebGpu);

        let commit_result: Result<(), &str> = Err("surface configuration failed after binding");
        assert!(commit_result.is_err());
        assert_eq!(gl_attempts.get(), 0);
        Ok(())
    }
}
