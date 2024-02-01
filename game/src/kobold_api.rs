use async_trait::async_trait;
use es::SSE;
use eventsource_client as es;
use futures::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;
use std::time::Duration;

use crate::ai::convo::AiCreativity;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

fn creativity_to_temperature(creativity: AiCreativity) -> Option<f64> {
    match creativity {
        AiCreativity::Predictable => Some(0.5),
        AiCreativity::Normal => Some(0.7),
        AiCreativity::Creative => Some(1.0),
    }
}

pub fn create_input(
    gen_key: String,
    prompt: &str,
    grammar: Option<String>,
    max_tokens: u64,
    retain_gramar_state: bool,
    creativity: AiCreativity,
) -> types::GenerationInput {
    types::GenerationInput {
        genkey: Some(gen_key),
        prompt: prompt.to_string(),
        grammar: grammar,
        grammar_retain_state: retain_gramar_state,
        use_default_badwordsids: false,
        max_context_length: None,
        max_length: NonZeroU64::new(max_tokens),
        min_p: None,
        mirostat: None,
        mirostat_eta: None,
        mirostat_tau: None,
        rep_pen: Some(1.1),
        temperature: creativity_to_temperature(creativity),
        tfs: None,
        top_a: Some(0.0),
        top_p: Some(0.92),
        typical: None,
        rep_pen_range: Some(320),
        top_k: None,
        sampler_order: vec![6, 0, 1, 3, 4, 2, 5],
        sampler_seed: None,
        stop_sequence: vec!["<s>".to_string(), "</s>".to_string()],
    }
}

pub struct WrappedGenerationError(String);

impl From<es::Error> for WrappedGenerationError {
    fn from(value: es::Error) -> Self {
        WrappedGenerationError(format!("{:?}", value))
    }
}

#[derive(Serialize, Deserialize)]
struct AIEvent {
    token: String,
}

fn create_response_stream(
    client: impl es::Client,
) -> impl Stream<Item = Result<String, es::Error>> {
    client.stream().map(|sse| {
        sse.and_then(|event| match event {
            SSE::Event(ev) => serde_json::from_str::<AIEvent>(&ev.data)
                .map(|r| r.token)
                .map_err(|err| es::Error::Unexpected(Box::new(err))),
            SSE::Comment(_) => Ok("".to_string()),
        })
    })
}

#[async_trait]
pub trait SseGenerationExt {
    async fn sse_generate(
        &self,
        input: types::GenerationInput,
    ) -> std::result::Result<String, es::Error>;
}

#[async_trait]
impl SseGenerationExt for Client {
    async fn sse_generate(
        &self,
        input: types::GenerationInput,
    ) -> std::result::Result<String, es::Error> {
        let params = serde_json::to_string(&input)?;
        let stream_url = format!("{}/extra/generate/stream", self.baseurl());

        let reconnect_opts = es::ReconnectOptions::reconnect(true)
            .retry_initial(false)
            .delay(Duration::from_secs(1))
            .backoff_factor(2)
            .delay_max(Duration::from_secs(60))
            .build();

        let client = es::ClientBuilder::for_url(&stream_url)?
            .header("accept", "application/json")?
            .header("Content-Type", "application/json")?
            .method("POST".to_string())
            .body(params)
            .reconnect(reconnect_opts)
            .build();

        let mut stream = create_response_stream(client);
        let mut response = String::new();

        loop {
            let maybe_token = stream.try_next().await;
            match maybe_token {
                Ok(Some(token)) => response.push_str(&token),
                Err(es::Error::Eof) => break,
                Err(err) => return Err(err),
                _ => (),
            }
        }

        Ok(response)
    }
}
