//! socket-metrics-exporter — Prometheus exporter for `ss -s` socket metrics.
//!
//! Runs an HTTP server exposing a `/metrics` endpoint. A background task
//! periodically shells out to `ss -s`, parses the output, and updates
//! Prometheus gauges.

use std::net::{self, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_web::rt;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use clap::Parser;
use prometheus::core::{AtomicU64, GenericGauge, GenericGaugeVec};
use prometheus::{Encoder, Opts, Registry, TextEncoder};
use socket_metrics_exporter::{SocketStats, collect};
use tokio::time;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Command line arguments
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// TCP host bind interface
    #[arg(short = 'i', long, default_value = "localhost", env = "SS_EXPORTER_HOST")]
    host: String,

    /// TCP port
    #[arg(short, long, default_value_t = 9186, env = "SS_EXPORTER_PORT")]
    port: u16,

    /// Interval between `ss -s` refreshes, in seconds
    #[arg(
        short = 'n',
        long,
        default_value_t = 15,
        env = "SS_EXPORTER_INTERVAL"
    )]
    interval_secs: u64,
}

impl net::ToSocketAddrs for Args {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        (self.host.clone(), self.port).to_socket_addrs()
    }
}

/// Application state shared across HTTP handlers.
#[derive(Clone)]
struct AppState {
    registry: Arc<Registry>,
}

/// Prometheus gauges populated from parsed `ss -s` output.
#[derive(Clone)]
struct SsGauges {
    sockets_total: GenericGauge<AtomicU64>,
    tcp_total: GenericGauge<AtomicU64>,
    tcp_by_state: GenericGaugeVec<AtomicU64>,
    transport: GenericGaugeVec<AtomicU64>,
    last_scrape_success: GenericGauge<AtomicU64>,
    last_scrape_timestamp: GenericGauge<AtomicU64>,
}

impl SsGauges {
    fn new() -> Self {
        let sockets_total = GenericGauge::<AtomicU64>::with_opts(Opts::new(
            "ss_sockets_total",
            "Total sockets reported by `ss -s` (Total: line).",
        ))
        .unwrap();

        let tcp_total = GenericGauge::<AtomicU64>::with_opts(Opts::new(
            "ss_tcp_sockets_total",
            "Total TCP sockets reported by `ss -s` (TCP: line).",
        ))
        .unwrap();

        let tcp_by_state = GenericGaugeVec::<AtomicU64>::new(
            Opts::new(
                "ss_tcp_sockets",
                "TCP sockets broken down by connection state from `ss -s`.",
            ),
            &["state"],
        )
        .unwrap();

        let transport = GenericGaugeVec::<AtomicU64>::new(
            Opts::new(
                "ss_transport_sockets",
                "Socket counts from the `ss -s` transport table, labelled by transport and address family.",
            ),
            &["transport", "family"],
        )
        .unwrap();

        let last_scrape_success = GenericGauge::<AtomicU64>::with_opts(Opts::new(
            "ss_last_scrape_success",
            "1 if the last `ss -s` invocation succeeded, 0 otherwise.",
        ))
        .unwrap();

        let last_scrape_timestamp = GenericGauge::<AtomicU64>::with_opts(Opts::new(
            "ss_last_scrape_timestamp_seconds",
            "Unix timestamp (seconds) of the last successful `ss -s` invocation.",
        ))
        .unwrap();

        Self {
            sockets_total,
            tcp_total,
            tcp_by_state,
            transport,
            last_scrape_success,
            last_scrape_timestamp,
        }
    }

    fn register(&self, registry: &Registry) {
        registry
            .register(Box::new(self.sockets_total.clone()))
            .unwrap();
        registry.register(Box::new(self.tcp_total.clone())).unwrap();
        registry
            .register(Box::new(self.tcp_by_state.clone()))
            .unwrap();
        registry.register(Box::new(self.transport.clone())).unwrap();
        registry
            .register(Box::new(self.last_scrape_success.clone()))
            .unwrap();
        registry
            .register(Box::new(self.last_scrape_timestamp.clone()))
            .unwrap();
    }

    fn apply(&self, stats: &SocketStats) {
        self.sockets_total.set(stats.total);
        self.tcp_total.set(stats.tcp.total);

        for (state, count) in &stats.tcp.states {
            self.tcp_by_state.with_label_values(&[state]).set(*count);
        }

        for (name, row) in &stats.transports {
            let transport_label = name.to_lowercase();
            self.transport
                .with_label_values(&[transport_label.as_str(), "total"])
                .set(row.total);
            self.transport
                .with_label_values(&[transport_label.as_str(), "ipv4"])
                .set(row.ipv4);
            self.transport
                .with_label_values(&[transport_label.as_str(), "ipv6"])
                .set(row.ipv6);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_scrape_timestamp.set(now);
        self.last_scrape_success.set(1);
    }
}

/// Background task that refreshes `SsGauges` from `ss -s` on a fixed interval.
async fn collector_loop(gauges: SsGauges, interval: Duration) {
    loop {
        match collect().await {
            Ok(stats) => gauges.apply(&stats),
            Err(e) => {
                error!("failed to collect ss -s metrics: {e}");
                gauges.last_scrape_success.set(0);
            }
        }
        time::sleep(interval).await;
    }
}

fn init_registry(args: &Args) -> Registry {
    let gauges = SsGauges::new();
    let registry = Registry::new();
    gauges.register(&registry);

    let interval = Duration::from_secs(args.interval_secs.max(1));
    rt::spawn(async move {
        collector_loop(gauges, interval).await;
    });

    registry
}

#[get("/metrics")]
async fn metrics(data: web::Data<AppState>) -> impl Responder {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = data.registry.gather();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        warn!("failed to encode metrics: {e}");
        return HttpResponse::InternalServerError().body("failed to encode metrics");
    }
    match String::from_utf8(buffer) {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(body),
        Err(e) => {
            warn!("metrics buffer was not valid UTF-8: {e}");
            HttpResponse::InternalServerError().body("metrics buffer was not valid UTF-8")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    info!("starting socket-metrics-exporter");

    let registry = init_registry(&args);

    let app_state = AppState {
        registry: Arc::new(registry),
    };

    info!(
        "serving /metrics on {}:{} (refresh every {}s)",
        args.host, args.port, args.interval_secs
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(metrics)
            .service(web::redirect("/", "/metrics"))
    })
    .bind(&args)?
    .run()
    .await
}
