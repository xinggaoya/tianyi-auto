use anyhow::{Context, Result, bail};
use chrono::{Local, TimeDelta};
use clap::Parser;
use cron::Schedule;
use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, HeaderMap, HeaderValue, PRAGMA, REFERER,
    USER_AGENT,
};
use reqwest::redirect::Policy;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "tianyi-auto", about = "Login then reboot Tianyi/ZTE router")]
struct Args {
    /// Router password (env: ROUTER_PASSWORD)
    #[arg(long, env = "ROUTER_PASSWORD")]
    password: String,
    /// Router username
    #[arg(long, default_value = "useradmin")]
    username: String,
    /// Router base URL (with scheme)
    #[arg(long, default_value = "http://192.168.1.1")]
    host: String,
    /// Login path
    #[arg(long, default_value = "/")]
    login_path: String,
    /// Reboot path
    #[arg(long, default_value = "/common_page/gatewayManage.lua")]
    reboot_path: String,
    /// Referer for reboot
    #[arg(long, default_value = "/common_page/main.lp")]
    reboot_referer: String,
    /// Login token value
    #[arg(long, default_value = "5")]
    login_token: String,
    /// frashnum value
    #[arg(long, default_value = "")]
    frashnum: String,
    /// Add timestamp query param on reboot
    #[arg(long, default_value_t = true)]
    reboot_timestamp: bool,
    /// Request timeout seconds
    #[arg(long, default_value_t = 10)]
    timeout_secs: u64,
    /// Cron expression for scheduled runs (local time). Default: Mon 04:00
    #[arg(long, default_value = "0 4 * * Mon")]
    cron: String,
    /// Run once immediately on start
    #[arg(long, default_value_t = false)]
    run_now: bool,
    /// Verbose logging
    #[arg(long, short, default_value_t = false)]
    verbose: bool,
}

#[derive(Debug)]
struct Config {
    login_url: Url,
    reboot_url: Url,
    reboot_referer: Url,
    username: String,
    password: String,
    login_token: String,
    frashnum: String,
    add_timestamp: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logger(args.verbose);

    let base = Url::parse(&args.host).context("invalid host URL")?;
    let cfg = Config {
        login_url: build_url(&base, &args.login_path)?,
        reboot_url: build_url(&base, &args.reboot_path)?,
        reboot_referer: build_url(&base, &args.reboot_referer)?,
        username: args.username,
        password: args.password,
        login_token: args.login_token,
        frashnum: args.frashnum,
        add_timestamp: args.reboot_timestamp,
    };

    let client = build_client(args.timeout_secs)?;

    run_scheduler(client, cfg, &args.cron, args.run_now)
}

fn init_logger(verbose: bool) {
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(""));
    builder.filter_level(level).format_timestamp_secs().init();
}

fn build_client(timeout_secs: u64) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36",
        ),
    );
    default_headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"),
    );
    default_headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
    );
    default_headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    default_headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    default_headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));

    Client::builder()
        .default_headers(default_headers)
        .cookie_store(true)
        .redirect(Policy::limited(4))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("building HTTP client")
}

fn login(client: &Client, cfg: &Config) -> Result<()> {
    let mut form: HashMap<String, String> = HashMap::new();
    form.insert("frashnum".into(), cfg.frashnum.clone());
    form.insert("action".into(), "login".into());
    form.insert("Frm_Logintoken".into(), cfg.login_token.clone());
    form.insert("user_name".into(), cfg.username.clone());
    form.insert("Password".into(), cfg.password.clone());

    let origin = origin_of(&cfg.login_url)?;
    let resp = client
        .post(cfg.login_url.clone())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Origin", origin.as_str())
        .header("Upgrade-Insecure-Requests", "1")
        .header(REFERER, cfg.login_url.as_str())
        .form(&form)
        .send()
        .context("login request failed")?;

    let status = resp.status();
    debug!("login status={}", status);

    if !status.is_success() {
        bail!("login failed with status {}", status);
    }

    let had_cookie = resp.cookies().next().is_some();
    if !had_cookie {
        warn!("No cookies received from login; device may still accept commands without cookie.");
    } else {
        debug!("Login cookies captured.");
    }

    Ok(())
}

fn reboot(client: &Client, cfg: &Config) -> Result<()> {
    let origin = origin_of(&cfg.reboot_url)?;
    let mut url = cfg.reboot_url.clone();
    if cfg.add_timestamp {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        url.query_pairs_mut()
            .append_pair("timeStamp", &ts.to_string());
    }

    let payload = json!({
        "RPCMethod": "Post",
        "Parameter": {
            "CmdType": "HG_COMMAND_REBOOT",
        }
    })
    .to_string();

    let resp = client
        .post(url)
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .header(ACCEPT, "application/json, text/javascript, */*; q=0.01")
        .header("Origin", origin.as_str())
        .header(REFERER, cfg.reboot_referer.as_str())
        .form(&[("jsonCfg", payload)])
        .send()
        .context("reboot request failed")?;

    let status = resp.status();
    debug!("reboot status={}", status);
    if !status.is_success() {
        bail!("reboot request returned {}", status);
    }

    Ok(())
}

fn run_scheduler(client: Client, cfg: Config, cron_expr: &str, run_now: bool) -> Result<()> {
    let schedule = Schedule::from_str(cron_expr).context("invalid cron expression")?;

    if run_now {
        info!("Running immediately due to --run-now");
        if let Err(e) = run_once(&client, &cfg) {
            error!("Immediate run failed: {e:?}");
        }
    }

    loop {
        let now = Local::now();
        let next = schedule
            .after(&now)
            .next()
            .context("cron produced no future times")?;
        let wait_delta = next - now;
        let wait = to_std(wait_delta);
        info!(
            "Next run at {} (in {:.1} minutes)",
            next,
            wait.as_secs_f64() / 60.0
        );
        thread::sleep(wait);
        if let Err(e) = run_once(&client, &cfg) {
            error!("Scheduled run failed: {e:?}");
        }
    }
}

fn run_once(client: &Client, cfg: &Config) -> Result<()> {
    login(client, cfg)?;
    info!("Login request sent.");
    reboot(client, cfg)?;
    info!("Reboot command dispatched.");
    Ok(())
}

fn to_std(delta: TimeDelta) -> Duration {
    if let Ok(d) = delta.to_std() {
        d
    } else {
        Duration::from_secs(0)
    }
}

fn origin_of(url: &Url) -> Result<Url> {
    let mut origin = url.clone();
    origin.set_path("");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin)
}

fn build_url(base: &Url, path: &str) -> Result<Url> {
    if path.starts_with("http://") || path.starts_with("https://") {
        Url::parse(path).context("invalid absolute url")
    } else {
        base.join(path).context("invalid relative url")
    }
}
