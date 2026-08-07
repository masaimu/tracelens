use std::collections::{BTreeMap, BTreeSet};

use crate::graph::trace_graph::TraceGraph;
use crate::model::span::{CanonicalSpan, SpanLink};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceAnnotations {
    pub counts: AnnotationCounts,
    pub spans: Vec<SpanAnnotation>,
    pub client_server_pairs: Vec<ClientServerPair>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AnnotationCounts {
    pub client_server_pairs: usize,
    pub client_server_span_count: usize,
    pub async_span_count: usize,
    pub linked_span_count: usize,
    pub messaging_span_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpanAnnotation {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub role: SpanRole,
    pub client_server_peers: Vec<ClientServerPeer>,
    pub async_work: bool,
    pub messaging: bool,
    pub linked_span_count: usize,
    pub linked_spans: Vec<LinkedSpanRef>,
    pub notes: Vec<AnnotationNote>,
}

impl SpanAnnotation {
    pub fn has_annotations(&self) -> bool {
        !self.client_server_peers.is_empty()
            || self.async_work
            || self.messaging
            || self.linked_span_count > 0
    }

    pub fn is_async_related(&self) -> bool {
        self.async_work || self.messaging || self.linked_span_count > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpanRole {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
    Unknown,
}

impl SpanRole {
    pub fn from_kind(kind: Option<i64>) -> Self {
        match kind {
            Some(1) => Self::Internal,
            Some(2) => Self::Server,
            Some(3) => Self::Client,
            Some(4) => Self::Producer,
            Some(5) => Self::Consumer,
            _ => Self::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Server => "server",
            Self::Client => "client",
            Self::Producer => "producer",
            Self::Consumer => "consumer",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientServerPair {
    pub client_span_id: String,
    pub client_service_name: String,
    pub client_name: String,
    pub server_span_id: String,
    pub server_service_name: String,
    pub server_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientServerPeer {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub relationship: ClientServerRelationship,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientServerRelationship {
    ClientToServer,
    ServerFromClient,
}

impl ClientServerRelationship {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClientToServer => "client_to_server",
            Self::ServerFromClient => "server_from_client",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedSpanRef {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub same_trace: bool,
    pub target_in_trace: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnotationNote {
    ClientServerPair,
    AsyncKind,
    MessagingAttributes,
    SpanLinks,
}

impl AnnotationNote {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClientServerPair => "client_server_pair",
            Self::AsyncKind => "async_kind",
            Self::MessagingAttributes => "messaging_attributes",
            Self::SpanLinks => "span_links",
        }
    }
}

pub fn annotate_trace_spans(trace: &TraceGraph) -> TraceAnnotations {
    let span_ids: BTreeSet<&str> = trace
        .spans
        .iter()
        .map(|span| span.span_id.as_str())
        .collect();
    let mut spans: Vec<SpanAnnotation> = trace
        .spans
        .iter()
        .map(|span| annotate_single_span(trace, span, &span_ids))
        .collect();

    let parent_indices = first_parent_indices(trace);
    let mut client_server_pairs = Vec::new();
    for (child_index, child) in trace.spans.iter().enumerate() {
        let Some(parent_index) = child
            .parent_span_id
            .as_deref()
            .and_then(|parent_span_id| parent_indices.get(parent_span_id).copied())
        else {
            continue;
        };
        let parent = &trace.spans[parent_index];
        if SpanRole::from_kind(parent.kind) != SpanRole::Client
            || SpanRole::from_kind(child.kind) != SpanRole::Server
        {
            continue;
        }

        client_server_pairs.push(ClientServerPair {
            client_span_id: parent.span_id.clone(),
            client_service_name: parent.service_name.clone(),
            client_name: parent.name.clone(),
            server_span_id: child.span_id.clone(),
            server_service_name: child.service_name.clone(),
            server_name: child.name.clone(),
        });

        spans[parent_index]
            .client_server_peers
            .push(ClientServerPeer {
                span_id: child.span_id.clone(),
                service_name: child.service_name.clone(),
                name: child.name.clone(),
                relationship: ClientServerRelationship::ClientToServer,
            });
        push_note_once(
            &mut spans[parent_index].notes,
            AnnotationNote::ClientServerPair,
        );

        spans[child_index]
            .client_server_peers
            .push(ClientServerPeer {
                span_id: parent.span_id.clone(),
                service_name: parent.service_name.clone(),
                name: parent.name.clone(),
                relationship: ClientServerRelationship::ServerFromClient,
            });
        push_note_once(
            &mut spans[child_index].notes,
            AnnotationNote::ClientServerPair,
        );
    }

    let counts = AnnotationCounts {
        client_server_pairs: client_server_pairs.len(),
        client_server_span_count: spans
            .iter()
            .filter(|span| !span.client_server_peers.is_empty())
            .count(),
        async_span_count: spans.iter().filter(|span| span.async_work).count(),
        linked_span_count: spans
            .iter()
            .filter(|span| span.linked_span_count > 0)
            .count(),
        messaging_span_count: spans.iter().filter(|span| span.messaging).count(),
    };

    TraceAnnotations {
        counts,
        spans,
        client_server_pairs,
    }
}

fn annotate_single_span(
    trace: &TraceGraph,
    span: &CanonicalSpan,
    span_ids: &BTreeSet<&str>,
) -> SpanAnnotation {
    let role = SpanRole::from_kind(span.kind);
    let messaging = has_messaging_attributes(span);
    let linked_spans = span
        .links
        .iter()
        .map(|link| linked_span_ref(trace, link, span_ids))
        .collect::<Vec<_>>();
    let linked_span_count = linked_spans.len();
    let async_work = matches!(role, SpanRole::Producer | SpanRole::Consumer)
        || messaging
        || linked_span_count > 0;
    let mut notes = Vec::new();
    if matches!(role, SpanRole::Producer | SpanRole::Consumer) {
        notes.push(AnnotationNote::AsyncKind);
    }
    if messaging {
        notes.push(AnnotationNote::MessagingAttributes);
    }
    if linked_span_count > 0 {
        notes.push(AnnotationNote::SpanLinks);
    }

    SpanAnnotation {
        span_id: span.span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        role,
        client_server_peers: Vec::new(),
        async_work,
        messaging,
        linked_span_count,
        linked_spans,
        notes,
    }
}

fn linked_span_ref(
    trace: &TraceGraph,
    link: &SpanLink,
    span_ids: &BTreeSet<&str>,
) -> LinkedSpanRef {
    let same_trace = link
        .trace_id
        .as_deref()
        .map(|trace_id| trace_id == trace.trace_id)
        .unwrap_or(false);
    let target_in_trace = link
        .span_id
        .as_deref()
        .is_some_and(|span_id| span_ids.contains(span_id))
        && (link.trace_id.is_none() || same_trace);

    LinkedSpanRef {
        trace_id: link.trace_id.clone(),
        span_id: link.span_id.clone(),
        same_trace,
        target_in_trace,
    }
}

fn has_messaging_attributes(span: &CanonicalSpan) -> bool {
    span.attributes
        .keys()
        .chain(span.resource_attributes.keys())
        .any(|key| key.starts_with("messaging."))
}

fn first_parent_indices(trace: &TraceGraph) -> BTreeMap<&str, usize> {
    let mut parent_by_span_id = BTreeMap::new();
    for (index, span) in trace.spans.iter().enumerate() {
        parent_by_span_id
            .entry(span.span_id.as_str())
            .or_insert(index);
    }
    parent_by_span_id
}

fn push_note_once(notes: &mut Vec<AnnotationNote>, note: AnnotationNote) {
    if !notes.contains(&note) {
        notes.push(note);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::annotations::{
        AnnotationNote, ClientServerRelationship, SpanRole, annotate_trace_spans,
    };
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::ParsedTraceData;
    use crate::model::span::{CanonicalSpan, SpanLink};

    #[test]
    fn annotates_client_server_async_and_linked_spans() {
        let collection = collection_with(vec![
            span("root", None, "frontend", "GET /checkout", Some(2), 0, 100),
            span(
                "client",
                Some("root"),
                "frontend",
                "GET inventory",
                Some(3),
                10,
                80,
            ),
            span(
                "server",
                Some("client"),
                "inventory",
                "GET /stock",
                Some(2),
                15,
                70,
            ),
            producer_span("producer", Some("server")),
            consumer_with_link("consumer", Some("root"), "producer"),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let annotations = annotate_trace_spans(trace);

        assert_eq!(annotations.counts.client_server_pairs, 1);
        assert_eq!(annotations.counts.client_server_span_count, 2);
        assert_eq!(annotations.counts.async_span_count, 2);
        assert_eq!(annotations.counts.messaging_span_count, 1);
        assert_eq!(annotations.counts.linked_span_count, 1);

        let pair = &annotations.client_server_pairs[0];
        assert_eq!(pair.client_span_id, "client");
        assert_eq!(pair.server_span_id, "server");

        let client = annotation(&annotations, "client");
        assert_eq!(client.role, SpanRole::Client);
        assert_eq!(client.client_server_peers.len(), 1);
        assert_eq!(
            client.client_server_peers[0].relationship,
            ClientServerRelationship::ClientToServer
        );

        let server = annotation(&annotations, "server");
        assert_eq!(server.role, SpanRole::Server);
        assert_eq!(
            server.client_server_peers[0].relationship,
            ClientServerRelationship::ServerFromClient
        );

        let producer = annotation(&annotations, "producer");
        assert!(producer.async_work);
        assert!(producer.messaging);
        assert!(producer.notes.contains(&AnnotationNote::AsyncKind));
        assert!(
            producer
                .notes
                .contains(&AnnotationNote::MessagingAttributes)
        );

        let consumer = annotation(&annotations, "consumer");
        assert!(consumer.async_work);
        assert_eq!(consumer.linked_span_count, 1);
        assert!(consumer.linked_spans[0].target_in_trace);
        assert!(consumer.notes.contains(&AnnotationNote::SpanLinks));
    }

    fn annotation<'a>(
        annotations: &'a crate::analysis::annotations::TraceAnnotations,
        span_id: &str,
    ) -> &'a crate::analysis::annotations::SpanAnnotation {
        annotations
            .spans
            .iter()
            .find(|annotation| annotation.span_id == span_id)
            .expect("annotation should exist")
    }

    fn collection_with(spans: Vec<CanonicalSpan>) -> TraceCollection {
        TraceCollection::build(ParsedTraceData {
            spans,
            diagnostics: Vec::new(),
        })
    }

    fn span(
        span_id: &str,
        parent_span_id: Option<&str>,
        service_name: &str,
        name: &str,
        kind: Option<i64>,
        start_ns: u64,
        end_ns: u64,
    ) -> CanonicalSpan {
        CanonicalSpan {
            trace_id: "trace".to_string(),
            span_id: span_id.to_string(),
            parent_span_id: parent_span_id.map(str::to_string),
            service_name: service_name.to_string(),
            name: name.to_string(),
            kind,
            start_ns,
            end_ns,
            status_code: None,
            attributes: BTreeMap::new(),
            resource_attributes: BTreeMap::new(),
            scope_name: None,
            scope_version: None,
            events: Vec::new(),
            links: Vec::new(),
        }
    }

    fn producer_span(span_id: &str, parent_span_id: Option<&str>) -> CanonicalSpan {
        let mut span = span(
            span_id,
            parent_span_id,
            "frontend",
            "publish checkout event",
            Some(4),
            72,
            90,
        );
        span.attributes
            .insert("messaging.system".to_string(), "kafka".to_string());
        span
    }

    fn consumer_with_link(
        span_id: &str,
        parent_span_id: Option<&str>,
        linked_span_id: &str,
    ) -> CanonicalSpan {
        let mut span = span(
            span_id,
            parent_span_id,
            "worker",
            "consume checkout event",
            Some(5),
            92,
            99,
        );
        span.links.push(SpanLink {
            trace_id: Some("trace".to_string()),
            span_id: Some(linked_span_id.to_string()),
            attributes: BTreeMap::new(),
        });
        span
    }
}
