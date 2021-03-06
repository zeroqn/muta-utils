//!

pub use muta_apm_derive as derive;
pub use rustracing;
pub use rustracing_jaeger;

use std::borrow::Cow;
use std::net::SocketAddr;

use parking_lot::RwLock;
use rustracing::sampler::AllSampler;
use rustracing::tag::Tag;
use rustracing_jaeger::reporter::JaegerCompactReporter;
use rustracing_jaeger::span::{Span, SpanContext};
use rustracing_jaeger::Tracer;

const SPAN_CHANNEL_SIZE: usize = 1024 * 1024;

lazy_static::lazy_static! {
    pub static ref MUTA_TRACER: MutaTracer = MutaTracer::new();
}

pub fn global_tracer_register(service_name: &str, udp_addr: SocketAddr) {
    let (span_tx, span_rx) = crossbeam_channel::bounded(SPAN_CHANNEL_SIZE);
    let mut reporter = JaegerCompactReporter::new(service_name).unwrap();
    let mut tracer = MUTA_TRACER.inner.write();
    *tracer = Some(Tracer::with_sender(AllSampler, span_tx));

    reporter
        .set_agent_addr(udp_addr)
        .expect("set upd addr error");

    std::thread::spawn(move || loop {
        if let Ok(finished_span) = span_rx.try_recv() {
            reporter.report(&[finished_span]).unwrap();
        }
    });
}

#[derive(Default)]
pub struct MutaTracer {
    pub(crate) inner: RwLock<Option<Tracer>>,
}

impl MutaTracer {
    pub fn new() -> Self {
        MutaTracer {
            inner: RwLock::new(None),
        }
    }

    pub fn child_of_span<N: Into<Cow<'static, str>>>(
        &self,
        opt_name: N,
        parent_ctx: SpanContext,
        tags: Vec<Tag>,
    ) -> Option<Span> {
        match self.inner.read().as_ref() {
            Some(inner) => {
                let mut span = inner.span(opt_name);
                for tag in tags.into_iter() {
                    span = span.tag(tag);
                }
                Some(span.child_of(&parent_ctx).start())
            }
            None => None,
        }
    }

    pub fn span<N: Into<Cow<'static, str>>>(&self, opt_name: N, tags: Vec<Tag>) -> Option<Span> {
        match self.inner.read().as_ref() {
            Some(inner) => {
                let mut span = inner.span(opt_name);
                for tag in tags.into_iter() {
                    span = span.tag(tag);
                }
                Some(span.start())
            }
            None => None,
        }
    }
}
