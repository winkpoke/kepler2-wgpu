//! Separate uniform-upload and successful-presentation snapshots.
//! Comparing small CPU state catches direct configuration edits as well as setters,
//! without invalidating a view for getters, no-op controls, or unrelated events.

#[derive(Debug)]
pub(crate) struct ViewInvalidation<S> {
    uploaded: Option<S>,
    presented: Option<S>,
}

impl<S> Default for ViewInvalidation<S> {
    fn default() -> Self {
        Self {
            uploaded: None,
            presented: None,
        }
    }
}

impl<S: PartialEq> ViewInvalidation<S> {
    pub fn needs_upload(&self, state: &S) -> bool {
        self.uploaded.as_ref() != Some(state)
    }

    pub fn needs_redraw(&self, state: &S) -> bool {
        self.presented.as_ref() != Some(state)
    }

    pub fn uploaded(&mut self, state: S) {
        self.uploaded = Some(state);
    }

    pub fn presented(&mut self, state: S) {
        self.presented = Some(state);
    }

    pub fn invalidate(&mut self) {
        self.uploaded = None;
        self.presented = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_does_not_consume_pending_presentation() {
        let mut changes = ViewInvalidation::default();
        assert!(changes.needs_upload(&1));
        assert!(changes.needs_redraw(&1));
        changes.uploaded(1);
        assert!(!changes.needs_upload(&1));
        assert!(changes.needs_redraw(&1));
        changes.presented(1);
        assert!(!changes.needs_redraw(&1));
        assert!(changes.needs_upload(&2));
        assert!(changes.needs_redraw(&2));
        changes.invalidate();
        assert!(changes.needs_upload(&1));
        assert!(changes.needs_redraw(&1));
    }
}
