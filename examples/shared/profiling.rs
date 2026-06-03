use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct ProfilingArgs {
    pub positional: Vec<String>,
}

pub fn init_from_env(default_filter: &'static str) -> ProfilingArgs {
    let mut positional = Vec::new();
    let mut trace_enabled = false;
    let mut tracy_enabled = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--trace" => trace_enabled = true,
            "--tracy" => tracy_enabled = true,
            _ => positional.push(arg),
        }
    }

    if trace_enabled || tracy_enabled {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| default_filter.into());
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_filter(env_filter.clone());
        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        if tracy_enabled {
            let tracy_layer = tracing_tracy::TracyLayer::default().with_filter(env_filter);
            subscriber.with(tracy_layer).init();
        } else {
            subscriber.init();
        }
    }

    ProfilingArgs { positional }
}
