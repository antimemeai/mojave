use audit_events::{Detail, EventKind, Outcome, ResourceRef, Tags};

use crate::emitter::Emitter;
use crate::error::AuditError;

#[must_use]
pub struct AuditGate<T> {
    inner: Option<T>,
    event_kind: EventKind,
    resource: ResourceRef,
    outcome: Outcome,
}

impl<T> AuditGate<T> {
    pub fn new(inner: T, event_kind: EventKind, resource: ResourceRef, outcome: Outcome) -> Self {
        Self {
            inner: Some(inner),
            event_kind,
            resource,
            outcome,
        }
    }

    pub fn resolve(
        mut self,
        emitter: &mut Emitter,
        actor: audit_events::Principal,
        tags: Tags,
        detail: Detail,
    ) -> Result<T, AuditError> {
        let event = audit_events::AuditEvent {
            envelope_version: 1,
            at: chrono::Utc::now(),
            monotonic_ns: None,
            actor,
            trace_id: None,
            event: self.event_kind,
            resource: self.resource.clone(),
            authorization: audit_events::Authorization::Allowed,
            outcome: self.outcome.clone(),
            tags,
            detail,
            blob_ref: None,
        };

        emitter.emit(event)?;
        self.inner
            .take()
            .ok_or_else(|| AuditError::BlobStore("AuditGate inner already consumed".into()))
    }

    pub fn resolve_with_blob(
        mut self,
        emitter: &mut Emitter,
        actor: audit_events::Principal,
        tags: Tags,
        detail: Detail,
        blob: &[u8],
        content_type: &str,
    ) -> Result<T, AuditError> {
        let event = audit_events::AuditEvent {
            envelope_version: 1,
            at: chrono::Utc::now(),
            monotonic_ns: None,
            actor,
            trace_id: None,
            event: self.event_kind,
            resource: self.resource.clone(),
            authorization: audit_events::Authorization::Allowed,
            outcome: self.outcome.clone(),
            tags,
            detail,
            blob_ref: None,
        };

        emitter.emit_with_blob(event, blob, content_type)?;
        self.inner
            .take()
            .ok_or_else(|| AuditError::BlobStore("AuditGate inner already consumed".into()))
    }

    pub fn event_kind(&self) -> EventKind {
        self.event_kind
    }
}

impl<T> Drop for AuditGate<T> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            if cfg!(debug_assertions) {
                panic!(
                    "AuditGate dropped without resolution — event {:?} was never emitted",
                    self.event_kind
                );
            } else {
                eprintln!(
                    "AUDIT WARNING: AuditGate dropped without resolution for {:?}",
                    self.event_kind
                );
            }
        }
    }
}
