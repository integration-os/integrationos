use axum::{body::Bytes, http::HeaderMap};
use criterion::{criterion_group, criterion_main, Criterion};
use gateway::{config::Config, mock_finalizer::MockFinalizer, server::Server, state::AppState};
use integrationos_domain::common::{
    encrypted_access_key::EncryptedAccessKey, encrypted_data::PASSWORD_LENGTH, AccessKey, Event,
};
use std::{collections::HashMap, hint::black_box, sync::Arc};
use tokio::runtime::Builder;

const KEY: &str = "id_test_1_Q71YUIZydcgSwJQNOUCHhaTMqmIvslIafF5LluORJfJKydMGELHtYe_ydtBIrVuomEnOZ4jfZQgtkqWxtG-s7vhbyir4kNjLyHKyDyh1SDubBMlhSI7Mq-M5RVtwnwFqZiOeUkIgHJFgcGQn0Plb1AkAAAAAAAAAAAAAAAAAAAAAAMwWY_9_oDOV75noniBViOVmVPUQqzcW8G3P8nuUD6Q";
const PASSWORD: &[u8; PASSWORD_LENGTH] = b"32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS";

fn create_event_benchmark(c: &mut Criterion) {
    c.bench_function("create and serialize event", |b| {
        let key = AccessKey::parse_str(KEY, PASSWORD).unwrap();
        let body = "hello world".to_owned();
        b.iter(|| {
            let event = black_box(Event::new(
                black_box(&key),
                black_box(&EncryptedAccessKey::parse(KEY).unwrap()),
                black_box("event.received"),
                black_box(HeaderMap::default()),
                black_box(body.clone()),
            ));
            let _ = black_box(serde_json::to_string(black_box(&event)).unwrap());
        })
    });
}

async fn handler(
    encrypted_access_key: EncryptedAccessKey<'_>,
    payload: Bytes,
    query: Option<HashMap<String, String>>,
    headers: HeaderMap,
    state: Arc<AppState>,
) {
    let _ = black_box(
        Server::handle_event(
            black_box(encrypted_access_key),
            black_box(payload),
            black_box(query),
            black_box(headers),
            black_box(state),
        )
        .await,
    )
    .unwrap();
}

fn response_benchmark(c: &mut Criterion) {
    c.bench_function("respond to emit", |b| {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Creating runtime failed");
        let access_key = EncryptedAccessKey::parse(KEY).unwrap();
        let config = Config::default();
        let state = Arc::new(AppState::new(config, Arc::new(MockFinalizer)));
        let payload = Bytes::from_static(b"{\"foo\":\"bar\",\"baz\":\"qux\"}");
        b.to_async(rt).iter(|| {
            handler(
                access_key.clone(),
                payload.clone(),
                None,
                HeaderMap::default(),
                state.clone(),
            )
        })
    });
}

criterion_group!(benches, create_event_benchmark, response_benchmark);
criterion_main!(benches);
