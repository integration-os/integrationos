use crate::{
    metrics::{EVENTS_HISTOGRAM, STAGE_HISTOGRAM, STAGE_LABEL, STATUS_LABEL},
    store::{ContextStore, ControlDataStore, EventStore},
};
use anyhow::Result;
use chrono::Utc;
use futures::{
    future::{self},
    FutureExt,
};
use integrationos_domain::{
    algebra::{PipelineExt, PipelineStatus},
    pipeline_context::PipelineStage,
    root_context::RootStage,
    Event,
    {
        extractor_context::Stage as ExtractorStage, middleware::Middleware, ExtractorContext,
        PipelineContext, RootContext, Transaction,
    },
};
use js_sandbox_ios::Script;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    pin, select,
    time::{interval, sleep, Instant},
};
use tracing::{debug, error, info, trace, warn};

const KEEP_ALIVE_INTERVAL_SECS: u64 = 10;
const RETRY_INTERVAL_MILLIS: u64 = 500;
const TICK_INTERVAL_MILLIS: u64 = 1000;

#[derive(Clone)]
pub struct Dispatcher<X, Y, Z>
where
    X: ContextStore + Sync + Send + 'static,
    Y: EventStore + Sync + Send + 'static,
    Z: ControlDataStore + Sync + Send + 'static,
{
    pub context_store: Arc<X>,
    pub event_store: Arc<Y>,
    pub control_data_store: Arc<Z>,
}

macro_rules! select_contexts {
    ($self:ident.$fn:ident($contexts:ident, $key:ident)) => {
        if !$contexts.is_empty() {
            let mut tasks = Vec::with_capacity($contexts.len());
            for context in $contexts.values() {
                tasks.push(Box::pin($self.$fn(context.clone())));
            }
            loop {
                let (task, _, remaining_tasks) = future::select_all(tasks).await;
                tasks = remaining_tasks;

                match task {
                    Ok(context) => {
                        $contexts.insert(context.$key.clone(), context);
                    }
                    Err(err) => {
                        error!("{err:?}");
                    }
                }

                if tasks.is_empty() {
                    break;
                }
            }
        }
    };
}

impl<X, Y, Z> Dispatcher<X, Y, Z>
where
    X: ContextStore + Sync + Send + 'static,
    Y: EventStore + Sync + Send + 'static,
    Z: ControlDataStore + Sync + Send + 'static,
{
    #[tracing::instrument(skip(self, context), fields(event_id = %context.event_key))]
    pub async fn process_context(&self, mut context: RootContext) -> Result<RootContext> {
        let time: Instant = Instant::now();
        info!("Processing event");
        let mut interval = interval(Duration::from_secs(KEEP_ALIVE_INTERVAL_SECS));
        'outer: loop {
            let fut = self.process_root_context(context.clone());
            pin!(fut);
            loop {
                select! {
                    res = &mut fut => {
                        match res {
                            Ok(new_context) => {
                                let should_save = match new_context.stage {
                                    RootStage::ProcessingPipelines(ref pipelines) => !pipelines.is_empty(),
                                    _ => true
                                };
                                context = new_context;
                                if should_save {
                                    context.timestamp = Utc::now();
                                    self.context_store.set(context.clone()).await?;
                                }
                            },
                            Err(e) =>  {
                                error!("Error processing root context: {e}");
                                sleep(Duration::from_millis(RETRY_INTERVAL_MILLIS)).await;
                            }
                        }

                        if context.is_complete() {
                            break 'outer;
                        }
                        continue 'outer;
                    },
                    _ = interval.tick() => {
                        context.timestamp = Utc::now();
                        self.context_store.set(context.clone()).await?;
                    }
                }
            }
        }
        let elapsed = Instant::now() - time;
        info!("Finished processing event in {:?}", elapsed);
        metrics::histogram!(EVENTS_HISTOGRAM, elapsed, STATUS_LABEL => context.status.to_string());
        Ok(context)
    }

    #[tracing::instrument(skip(self, context), fields(stage = %context.stage))]
    pub async fn process_root_context(&self, mut context: RootContext) -> Result<RootContext> {
        let time: Instant = Instant::now();
        trace!("Processing root context {}", context.event_key);
        let event = self.event_store.get(&context.event_key).await?;
        trace!("Retrieved event {event:?}");

        let context = match context.stage {
            RootStage::New => {
                debug!("Verifying event");
                let verified = self.verify_event(&event).await?;
                if verified {
                    trace!("Event successfully verified");
                    context.stage = RootStage::Verified;
                } else {
                    warn!("Event did not verify, dropped");
                    context.status = PipelineStatus::Dropped {
                        reason: "Did not verify".to_owned(),
                    };
                }
                context
            }
            RootStage::Verified => {
                debug!("Fetching duplicates");
                let mut context = self.fetch_duplicates(context, event).await?;
                trace!("Duplicates fetched");
                context.stage = RootStage::ProcessedDuplicates;
                context
            }
            RootStage::ProcessedDuplicates => {
                debug!("Getting pipelines");
                let pipelines = self.control_data_store.get_pipelines(&event).await?;
                let pipelines: HashMap<_, _> = pipelines
                    .into_iter()
                    .map(|p| (p.key.clone(), PipelineContext::new(p.key, &context)))
                    .collect();
                trace!("Got {} pipelines", pipelines.len());
                context.stage = RootStage::ProcessingPipelines(pipelines);
                context
            }
            RootStage::ProcessingPipelines(ref mut pipelines) => {
                debug!("Processing pipelines");
                select_contexts!(self.process_pipeline(pipelines, pipeline_key));
                trace!("Processed pipelines");
                context.stage = RootStage::Finished;
                context
            }
            RootStage::Finished => {
                trace!("Finished root context");
                context
            }
        };
        let elapsed = Instant::now() - time;
        trace!("Finished processing root context in {:?}", elapsed);
        metrics::histogram!(STAGE_HISTOGRAM, elapsed, STAGE_LABEL => context.stage.to_string());
        Ok(context)
    }

    #[tracing::instrument(skip(self, context), fields(pipeline_key = %context.pipeline_key))]
    pub async fn process_pipeline(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        debug!("Processing pipeline");
        loop {
            let time = Instant::now();
            context = self.process_pipeline_context(context).await?;
            let elapsed = Instant::now() - time;
            trace!("Finished processing pipeline context in {:?}", elapsed);
            metrics::histogram!(STAGE_HISTOGRAM, elapsed, STAGE_LABEL => context.stage.to_string());
            let should_save = match context.stage {
                PipelineStage::ExecutingExtractors(ref extractors) => !extractors.is_empty(),
                PipelineStage::ExecutedExtractors(ref contexts) => !contexts.is_empty(),
                PipelineStage::ExecutedTransformer(ref context) => context.is_some(),
                _ => true,
            };
            if should_save {
                context.timestamp = Utc::now();
                self.context_store.set(context.clone()).await?;
                context.transaction = None;
            }
            if context.is_complete() {
                break;
            }
        }
        trace!("Processed pipeline");
        Ok(context)
    }

    #[tracing::instrument(skip(self, context), fields(stage = %context.stage))]
    pub async fn process_pipeline_context(
        &self,
        mut context: PipelineContext,
    ) -> Result<PipelineContext> {
        trace!("Processing pipeline context");
        let pipeline = self
            .control_data_store
            .get_pipeline(&context.pipeline_key)
            .await?;
        trace!("Retrieved pipeline {pipeline:?}");
        let event = self.event_store.get(&context.event_key).await?;

        match context.stage {
            PipelineStage::New => {
                debug!("Getting extractors");
                let extractors: HashMap<String, ExtractorContext> = pipeline
                    .middleware
                    .into_iter()
                    .filter_map(|e| match e {
                        Middleware::HttpExtractor(e) => {
                            Some((e.key.clone(), ExtractorContext::new(e.key, &context)))
                        }
                        Middleware::Transformer { .. } => None,
                    })
                    .collect();
                trace!("Got {} extractors", extractors.len());
                if extractors.is_empty() {
                    context.stage = PipelineStage::ExecutedExtractors(HashMap::new());
                } else {
                    context.stage = PipelineStage::ExecutingExtractors(extractors);
                }
                Ok(context)
            }
            PipelineStage::ExecutingExtractors(ref mut extractors) => {
                debug!("Processing extractors");
                select_contexts!(self.process_extractor(extractors, extractor_key));
                trace!("Processed extractors");
                let mut contexts = HashMap::with_capacity(extractors.len());
                for e in extractors.values() {
                    match e.stage {
                        ExtractorStage::New => {
                            return Ok(context);
                        }
                        ExtractorStage::FinishedExtractor(ref context) => {
                            contexts.insert(e.extractor_key.clone(), context.clone());
                        }
                    }
                }
                context.stage = PipelineStage::ExecutedExtractors(contexts);

                Ok(context)
            }
            PipelineStage::ExecutedExtractors(contexts) => {
                debug!("Executing transformer");
                let Some(Middleware::Transformer { code, .. }) = pipeline
                    .middleware
                    .iter()
                    .find(|m| matches!(m, Middleware::Transformer { .. }))
                else {
                    trace!("Did not find transformer, sending directly to destination");
                    context.stage = PipelineStage::ExecutedTransformer(if contexts.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_value(contexts)?)
                    });
                    return Ok(context);
                };

                let mut script = Script::from_string(code.as_str())?
                    .with_timeout(Duration::from_secs(TICK_INTERVAL_MILLIS));
                let value: Value = script
                    .call("transform", (event.clone(), contexts))
                    .inspect_err(|_| {
                        error!("Failed to transform data with contexts");
                    })?;

                trace!("Executed transformer");
                context.transaction = Some(Transaction::completed(
                    &event,
                    format!("{}::transformer", pipeline.key),
                    "['{{event}}', '{{context}}']".to_owned(),
                    value.to_string(),
                ));
                context.stage = PipelineStage::ExecutedTransformer(Some(value));
                Ok(context)
            }
            PipelineStage::ExecutedTransformer(ref value) => {
                debug!("Sending to destination");

                let retry = &pipeline.config.clone().unwrap_or_default().policies.retry;
                let retry_interval = retry.get_interval().unwrap_or(Duration::from_secs(1));
                let mut interval =
                    tokio::time::interval(Duration::from_millis(TICK_INTERVAL_MILLIS));
                'outer: for i in 0..retry.maximum_attempts {
                    let fut = self.control_data_store.send_to_destination(
                        &event,
                        &pipeline,
                        value.clone(),
                    );
                    pin!(fut);
                    loop {
                        select! {
                            res = &mut fut => {
                                let tx_key = if i > 0 {
                                    format!("{}::destination::attempt-{i}", pipeline.key)
                                } else {
                                    format!("{}::destination", pipeline.key)
                                };
                                let input = json!(["{{event}}", "{{context}}"]).to_string();
                                match res {
                                    Ok(value) => {
                                        trace!("Sent to destination");
                                        context.transaction = Some(Transaction::completed(
                                            &event,
                                            tx_key,
                                            input,
                                            value,
                                        ));
                                        context.stage = PipelineStage::FinishedPipeline;
                                        return Ok(context);
                                    }
                                    Err(e) => {
                                        error!("Failed to send to destination: {e}");
                                        if i < retry.maximum_attempts - 1 {
                                            context.transaction = Some(Transaction::failed(
                                                &event,
                                                tx_key,
                                                input,
                                                e.to_string(),
                                            ));
                                            context.timestamp = Utc::now();
                                            self.context_store.set(context.clone()).await?;
                                            context.transaction = None;
                                            sleep(retry_interval).await;
                                        } else {
                                            context.transaction = Some(Transaction::panicked(
                                                &event,
                                                tx_key,
                                                input,
                                                e.to_string(),
                                            ));
                                        }
                                        continue 'outer;
                                    }
                                }
                            },
                            _ = interval.tick() => {
                                context.transaction = Some(Transaction::completed(
                                    &event,
                                    format!("{}::heartbeat-{}", pipeline.key, i + 1),
                                    "['{{event}}', '{{context}}']".to_owned(),
                                    "{}".to_owned(),
                                ));
                                context.timestamp = Utc::now();
                                self.context_store.set(context.clone()).await?;
                            }
                        }
                    }
                }
                context.status = PipelineStatus::Dropped {
                    reason: "Failed destination".to_string(),
                };
                warn!("Failed destination");
                Ok(context)
            }
            PipelineStage::FinishedPipeline => {
                debug!("Executed pipeline");
                Ok(context)
            }
        }
    }

    #[tracing::instrument(skip(self, context), fields(extractor_key = %context.extractor_key))]
    pub async fn process_extractor(
        &self,
        mut context: ExtractorContext,
    ) -> Result<ExtractorContext> {
        trace!("Processing extractor");
        let extractor = self
            .control_data_store
            .get_extractor(&context.extractor_key, &context.pipeline_key)
            .await?;
        trace!("Retrieved extractor");

        let retry = &extractor.policies.retry;
        let retry_interval = retry.get_interval().unwrap_or(Duration::from_secs(1));
        let max_attempts = retry.maximum_attempts;

        let mut tick_interval = interval(Duration::from_millis(TICK_INTERVAL_MILLIS));
        'outer: for i in 0..max_attempts {
            let fut = self.control_data_store.execute_extractor(&extractor).fuse();
            pin!(fut);
            loop {
                select! {
                    res = &mut fut => {
                        let event = self.event_store.get(&context.event_key).await?;
                        let tx_key = if i > 0 {
                            format!("{}::extractor:http::attempt-{i}", extractor.key)
                        } else {
                            format!("{}::extractor:http", extractor.key)
                        };
                        let input = json!(["{{event}}"]).to_string();
                        match res {
                            Ok(value) => {
                                context.transaction = Some(Transaction::completed(
                                    &event,
                                    tx_key,
                                    input,
                                    serde_json::to_string(&value)?,
                                ));
                                context.stage = ExtractorStage::FinishedExtractor(value);
                                trace!("Executed extractor");
                                self.context_store.set(context.clone()).await?;
                                trace!("Saved extractor context");
                                return Ok(context);
                            }
                            Err(e) => {
                                if i < max_attempts - 1 {
                                    context.transaction = Some(Transaction::failed(
                                        &event,
                                        tx_key,
                                        input,
                                        e.to_string(),
                                    ));
                                    context.timestamp = Utc::now();
                                    self.context_store.set(context.clone()).await?;
                                    context.transaction = None;
                                    sleep(retry_interval).await;
                                } else {
                                    context.transaction = Some(Transaction::panicked(
                                        &event,
                                        tx_key,
                                        input,
                                        e.to_string(),
                                    ));
                                }
                                continue 'outer;
                            }
                        }
                    },
                    _ = tick_interval.tick() => {
                        context.transaction = None;
                        context.timestamp = Utc::now();
                        self.context_store.set(context.clone()).await?;
                    }
                }
            }
        }

        context.status = PipelineStatus::Dropped {
            reason: "Failed extractor".to_string(),
        };
        self.context_store.set(context.clone()).await?;
        warn!("Failed extractor");
        trace!("Saved failed extractor context");
        Ok(context)
    }

    #[tracing::instrument(skip(self, context, _event))]
    async fn fetch_duplicates(&self, context: RootContext, _event: Event) -> Result<RootContext> {
        // Disable duplicate detection for now
        // let duplicates = self.event_store.get_duplicates(&event).await?;
        // let mut event = event.add_duplicates(duplicates);
        // event.state = EventState::Acknowledged;
        // self.event_store.set(event).await?;
        Ok(context)
    }

    #[tracing::instrument(skip(self, event))]
    async fn verify_event(&self, event: &Event) -> Result<bool> {
        self.control_data_store.verify_event(event).await
    }
}
