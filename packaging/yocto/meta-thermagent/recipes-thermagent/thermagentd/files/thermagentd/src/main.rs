use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY: &str = "/etc/thermagent/policy.d/jetson-nano-lite.policy";
const REQUIRED_POLICY_KEYS: &[&str] = &[
    "name",
    "sample_interval_ms",
    "max_temp_c",
    "critical_temp_c",
    "min_fps",
    "workload_stale_after_s",
    "idle_nvpmodel_mode",
    "active_nvpmodel_mode",
    "hot_nvpmodel_mode",
    "idle_cpu_governor",
    "active_cpu_governor",
    "hot_cpu_governor",
    "nvpmodel_bin",
    "workload_file",
    "prometheus_listen",
];
const OPTIONAL_POLICY_KEYS: &[&str] = &[
    "thermal_hysteresis_c",
    "critical_hysteresis_c",
    "decision_hold_s",
    "missing_thermal_is_hot",
    "state_file",
    "allowed_cpu_governors",
    "allowed_nvpmodel_modes",
];

#[derive(Debug, Clone)]
struct Policy {
    name: String,
    sample_interval_ms: u64,
    max_temp_c: f64,
    critical_temp_c: f64,
    min_fps: f64,
    workload_stale_after_s: u64,
    idle_nvpmodel_mode: u8,
    active_nvpmodel_mode: u8,
    hot_nvpmodel_mode: u8,
    idle_cpu_governor: String,
    active_cpu_governor: String,
    hot_cpu_governor: String,
    nvpmodel_bin: String,
    workload_file: String,
    prometheus_listen: String,
    thermal_hysteresis_c: f64,
    critical_hysteresis_c: f64,
    decision_hold_s: u64,
    missing_thermal_is_hot: bool,
    state_file: String,
    allowed_cpu_governors: Vec<String>,
    allowed_nvpmodel_modes: Vec<u8>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            name: "jetson-nano-lite-safe-vision".to_string(),
            sample_interval_ms: 1000,
            max_temp_c: 72.0,
            critical_temp_c: 78.0,
            min_fps: 12.0,
            workload_stale_after_s: 15,
            idle_nvpmodel_mode: 1,
            active_nvpmodel_mode: 0,
            hot_nvpmodel_mode: 1,
            idle_cpu_governor: "schedutil".to_string(),
            active_cpu_governor: "schedutil".to_string(),
            hot_cpu_governor: "powersave".to_string(),
            nvpmodel_bin: "/usr/sbin/nvpmodel".to_string(),
            workload_file: "/run/thermagent/workload.metrics".to_string(),
            prometheus_listen: "127.0.0.1:9920".to_string(),
            thermal_hysteresis_c: 3.0,
            critical_hysteresis_c: 3.0,
            decision_hold_s: 5,
            missing_thermal_is_hot: true,
            state_file: "/var/lib/thermagent/state.json".to_string(),
            allowed_cpu_governors: Vec::new(),
            allowed_nvpmodel_modes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Args {
    config: String,
    dry_run: bool,
    once: bool,
    interval_ms: Option<u64>,
    prometheus_listen: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ThermalZone {
    name: String,
    temp_c: f64,
}

#[derive(Debug, Clone, Default)]
struct Telemetry {
    ts_unix: u64,
    max_temp_c: Option<f64>,
    thermal_zones: Vec<ThermalZone>,
    avg_cpu_freq_khz: Option<f64>,
    cpu_governors: BTreeMap<String, String>,
    gpu_cur_freq_hz: Option<u64>,
    gpu_max_freq_hz: Option<u64>,
    power_mw_sum: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct WorkloadHint {
    active: bool,
    fps: Option<f64>,
    phase: Option<String>,
    stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Decision {
    reason: String,
    nvpmodel_mode: u8,
    cpu_governor: String,
}

#[derive(Debug, Clone, Default)]
struct DecisionState {
    last_decision: Option<Decision>,
    last_change_unix: u64,
    thermal_guard_latched: bool,
    critical_cooldown_latched: bool,
}

#[derive(Debug, Clone, Default)]
struct SharedMetrics {
    ts_unix: u64,
    policy_name: String,
    reason: String,
    dry_run: bool,
    max_temp_c: Option<f64>,
    thermal_zones: Vec<ThermalZone>,
    avg_cpu_freq_khz: Option<f64>,
    cpu_governors: BTreeMap<String, String>,
    gpu_cur_freq_hz: Option<u64>,
    gpu_max_freq_hz: Option<u64>,
    power_mw_sum: Option<f64>,
    workload_active: bool,
    workload_fps: Option<f64>,
    workload_phase: Option<String>,
    workload_stale: bool,
    nvpmodel_mode: Option<u8>,
    cpu_governor: Option<String>,
    actions_failed: u64,
}

fn main() {
    let args = parse_args_or_exit();
    let mut policy = match read_policy(&args.config) {
        Ok(p) => p,
        Err(err) => {
            eprintln!(
                "thermagentd: failed to read policy {}: {}",
                args.config, err
            );
            std::process::exit(2);
        }
    };

    if let Some(ms) = args.interval_ms {
        policy.sample_interval_ms = ms;
    }
    if let Some(addr) = args.prometheus_listen.clone() {
        policy.prometheus_listen = addr;
    }

    println!(
        "thermagentd: starting policy={} config={} dry_run={} once={} interval_ms={}",
        policy.name, args.config, args.dry_run, args.once, policy.sample_interval_ms
    );

    let shared = Arc::new(Mutex::new(SharedMetrics {
        policy_name: policy.name.clone(),
        dry_run: args.dry_run,
        ..SharedMetrics::default()
    }));

    if !policy.prometheus_listen.trim().is_empty() {
        let metrics = Arc::clone(&shared);
        let addr = policy.prometheus_listen.clone();
        thread::spawn(move || {
            if let Err(err) = serve_metrics(addr, metrics) {
                eprintln!("thermagentd: prometheus server exited: {}", err);
            }
        });
    }

    let mut last_nvpmodel_mode: Option<u8> = None;
    let mut actions_failed: u64 = 0;
    let mut decision_state = DecisionState::default();

    loop {
        let telemetry = collect_telemetry();
        let workload = read_workload_hint(&policy.workload_file, policy.workload_stale_after_s);
        let decision = decide(&policy, &telemetry, &workload, &mut decision_state);

        println!(
            "thermagentd: reason={} temp={:?} fps={:?} active={} nvpmodel={} cpu_governor={}",
            decision.reason,
            telemetry.max_temp_c,
            workload.fps,
            workload.active,
            decision.nvpmodel_mode,
            decision.cpu_governor
        );

        match apply_decision(&policy, &decision, args.dry_run, &mut last_nvpmodel_mode) {
            Ok(()) => {}
            Err(err) => {
                actions_failed += 1;
                eprintln!("thermagentd: action failure: {}", err);
            }
        }

        if let Err(err) = write_state_file(
            &policy.state_file,
            &policy,
            &telemetry,
            &workload,
            &decision,
            actions_failed,
        ) {
            eprintln!("thermagentd: state write failed: {}", err);
        }

        if let Ok(mut guard) = shared.lock() {
            *guard = SharedMetrics {
                ts_unix: telemetry.ts_unix,
                policy_name: policy.name.clone(),
                reason: decision.reason.clone(),
                dry_run: args.dry_run,
                max_temp_c: telemetry.max_temp_c,
                thermal_zones: telemetry.thermal_zones,
                avg_cpu_freq_khz: telemetry.avg_cpu_freq_khz,
                cpu_governors: telemetry.cpu_governors,
                gpu_cur_freq_hz: telemetry.gpu_cur_freq_hz,
                gpu_max_freq_hz: telemetry.gpu_max_freq_hz,
                power_mw_sum: telemetry.power_mw_sum,
                workload_active: workload.active,
                workload_fps: workload.fps,
                workload_phase: workload.phase,
                workload_stale: workload.stale,
                nvpmodel_mode: Some(decision.nvpmodel_mode),
                cpu_governor: Some(decision.cpu_governor.clone()),
                actions_failed,
            };
        }

        if args.once {
            break;
        }
        thread::sleep(Duration::from_millis(policy.sample_interval_ms));
    }
}

fn parse_args_or_exit() -> Args {
    let mut args = Args {
        config: DEFAULT_POLICY.to_string(),
        dry_run: false,
        once: false,
        interval_ms: None,
        prometheus_listen: None,
    };

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--config" | "-c" => args.config = required_value(&mut iter, "--config"),
            "--dry-run" => args.dry_run = true,
            "--once" => args.once = true,
            "--interval-ms" => {
                let value = required_value(&mut iter, "--interval-ms");
                args.interval_ms = Some(value.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!("thermagentd: invalid --interval-ms value {}", value);
                    std::process::exit(2);
                }));
            }
            "--prometheus" => {
                args.prometheus_listen = Some(required_value(&mut iter, "--prometheus"))
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("thermagentd: unknown argument {}", other);
                print_help();
                std::process::exit(2);
            }
        }
    }
    args
}

fn required_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> String {
    iter.next().unwrap_or_else(|| {
        eprintln!("thermagentd: {} requires a value", flag);
        std::process::exit(2);
    })
}

fn print_help() {
    println!(
        "thermagentd 0.1.0\n\n\
         Usage:\n  thermagentd [--config PATH] [--dry-run] [--once] [--interval-ms N] [--prometheus ADDR]\n\n\
         Defaults:\n  --config {}\n  --prometheus 127.0.0.1:9920\n",
        DEFAULT_POLICY
    );
}

fn read_policy(path: &str) -> Result<Policy, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut policy = Policy::default();
    let mut seen = BTreeSet::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let no_comment = raw_line.split('#').next().unwrap_or("").trim();
        if no_comment.is_empty() || no_comment == "---" {
            continue;
        }

        let (key, value) = if let Some(pos) = no_comment.find(':') {
            (&no_comment[..pos], &no_comment[pos + 1..])
        } else if let Some(pos) = no_comment.find('=') {
            (&no_comment[..pos], &no_comment[pos + 1..])
        } else {
            return Err(format!("line {}: expected key: value", idx + 1));
        };

        let key = key.trim();
        let value = unquote(value.trim());
        match key {
            "name" => policy.name = value,
            "sample_interval_ms" => policy.sample_interval_ms = parse_u64(key, &value)?,
            "max_temp_c" => policy.max_temp_c = parse_f64(key, &value)?,
            "critical_temp_c" => policy.critical_temp_c = parse_f64(key, &value)?,
            "min_fps" => policy.min_fps = parse_f64(key, &value)?,
            "workload_stale_after_s" => policy.workload_stale_after_s = parse_u64(key, &value)?,
            "idle_nvpmodel_mode" => policy.idle_nvpmodel_mode = parse_u8(key, &value)?,
            "active_nvpmodel_mode" => policy.active_nvpmodel_mode = parse_u8(key, &value)?,
            "hot_nvpmodel_mode" => policy.hot_nvpmodel_mode = parse_u8(key, &value)?,
            "idle_cpu_governor" => policy.idle_cpu_governor = value,
            "active_cpu_governor" => policy.active_cpu_governor = value,
            "hot_cpu_governor" => policy.hot_cpu_governor = value,
            "nvpmodel_bin" => policy.nvpmodel_bin = value,
            "workload_file" => policy.workload_file = value,
            "prometheus_listen" => policy.prometheus_listen = value,
            "thermal_hysteresis_c" => policy.thermal_hysteresis_c = parse_f64(key, &value)?,
            "critical_hysteresis_c" => policy.critical_hysteresis_c = parse_f64(key, &value)?,
            "decision_hold_s" => policy.decision_hold_s = parse_u64(key, &value)?,
            "missing_thermal_is_hot" => policy.missing_thermal_is_hot = parse_bool(key, &value)?,
            "state_file" => policy.state_file = value,
            "allowed_cpu_governors" => policy.allowed_cpu_governors = parse_string_list(&value),
            "allowed_nvpmodel_modes" => policy.allowed_nvpmodel_modes = parse_u8_list(key, &value)?,
            unknown => return Err(format!("line {}: unknown key {}", idx + 1, unknown)),
        }
        seen.insert(key.to_string());
    }

    let missing: Vec<&str> = REQUIRED_POLICY_KEYS
        .iter()
        .copied()
        .filter(|key| !seen.contains(*key))
        .collect();
    if !missing.is_empty() {
        return Err(format!("missing policy keys: {}", missing.join(", ")));
    }
    let known: BTreeSet<&str> = REQUIRED_POLICY_KEYS
        .iter()
        .chain(OPTIONAL_POLICY_KEYS.iter())
        .copied()
        .collect();
    let unsupported: Vec<&str> = seen
        .iter()
        .map(String::as_str)
        .filter(|key| !known.contains(key))
        .collect();
    if !unsupported.is_empty() {
        return Err(format!(
            "unsupported policy keys: {}",
            unsupported.join(", ")
        ));
    }
    validate_policy(&policy)?;
    Ok(policy)
}

fn validate_policy(policy: &Policy) -> Result<(), String> {
    if policy.name.trim().is_empty() {
        return Err("name must not be empty".to_string());
    }
    if policy.sample_interval_ms < 100 {
        return Err("sample_interval_ms must be >= 100".to_string());
    }
    if !policy.max_temp_c.is_finite() {
        return Err("max_temp_c must be a finite number".to_string());
    }
    if !policy.critical_temp_c.is_finite() {
        return Err("critical_temp_c must be a finite number".to_string());
    }
    if policy.critical_temp_c <= policy.max_temp_c {
        return Err("critical_temp_c must be greater than max_temp_c".to_string());
    }
    if !policy.min_fps.is_finite() || policy.min_fps < 0.0 {
        return Err("min_fps must be a finite number >= 0".to_string());
    }
    if policy.workload_stale_after_s == 0 {
        return Err("workload_stale_after_s must be >= 1".to_string());
    }
    if !policy.thermal_hysteresis_c.is_finite() || policy.thermal_hysteresis_c < 0.0 {
        return Err("thermal_hysteresis_c must be a finite number >= 0".to_string());
    }
    if !policy.critical_hysteresis_c.is_finite() || policy.critical_hysteresis_c < 0.0 {
        return Err("critical_hysteresis_c must be a finite number >= 0".to_string());
    }
    if policy.state_file.trim().is_empty() {
        return Err("state_file must not be empty".to_string());
    }
    for (key, value) in [
        ("idle_cpu_governor", &policy.idle_cpu_governor),
        ("active_cpu_governor", &policy.active_cpu_governor),
        ("hot_cpu_governor", &policy.hot_cpu_governor),
        ("nvpmodel_bin", &policy.nvpmodel_bin),
        ("workload_file", &policy.workload_file),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{} must not be empty", key));
        }
    }
    for governor in &policy.allowed_cpu_governors {
        if governor.trim().is_empty() {
            return Err("allowed_cpu_governors must not contain empty values".to_string());
        }
    }
    for governor in [
        &policy.idle_cpu_governor,
        &policy.active_cpu_governor,
        &policy.hot_cpu_governor,
    ] {
        if !policy.allowed_cpu_governors.is_empty()
            && !policy
                .allowed_cpu_governors
                .iter()
                .any(|allowed| allowed == governor)
        {
            return Err(format!(
                "{} is not listed in allowed_cpu_governors",
                governor
            ));
        }
    }
    for mode in [
        policy.idle_nvpmodel_mode,
        policy.active_nvpmodel_mode,
        policy.hot_nvpmodel_mode,
    ] {
        if !policy.allowed_nvpmodel_modes.is_empty()
            && !policy
                .allowed_nvpmodel_modes
                .iter()
                .any(|allowed| *allowed == mode)
        {
            return Err(format!(
                "nvpmodel mode {} is not listed in allowed_nvpmodel_modes",
                mode
            ));
        }
    }
    Ok(())
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn parse_u64(key: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{} expects unsigned integer, got {}", key, value))
}

fn parse_u8(key: &str, value: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("{} expects u8 integer, got {}", key, value))
}

fn parse_f64(key: &str, value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{} expects number, got {}", key, value))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(format!("{} expects finite number, got {}", key, value))
    }
}

fn parse_bool(key: &str, value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("{} expects boolean, got {}", key, value)),
    }
}

fn parse_string_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_u8_list(key: &str, value: &str) -> Result<Vec<u8>, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| parse_u8(key, item))
        .collect()
}

fn collect_telemetry() -> Telemetry {
    let thermal_zones = read_thermal_zones();
    let max_temp_c = thermal_zones.iter().map(|z| z.temp_c).fold(None, |acc, t| {
        Some(match acc {
            Some(prev) if prev > t => prev,
            _ => t,
        })
    });
    let (avg_cpu_freq_khz, cpu_governors) = read_cpu_state();
    let (gpu_cur_freq_hz, gpu_max_freq_hz) = read_gpu_devfreq();
    let power_mw_sum = read_ina3221_power_mw();

    Telemetry {
        ts_unix: now_unix(),
        max_temp_c,
        thermal_zones,
        avg_cpu_freq_khz,
        cpu_governors,
        gpu_cur_freq_hz,
        gpu_max_freq_hz,
        power_mw_sum,
    }
}

fn read_thermal_zones() -> Vec<ThermalZone> {
    let mut zones = Vec::new();
    let base = Path::new("/sys/class/thermal");
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return zones,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let path = entry.path();
        let zone_type = read_trim(path.join("type")).unwrap_or(name);
        if let Some(raw_temp) = read_f64_file(path.join("temp")) {
            let temp_c = if raw_temp > 1000.0 {
                raw_temp / 1000.0
            } else {
                raw_temp
            };
            zones.push(ThermalZone {
                name: zone_type,
                temp_c,
            });
        }
    }
    zones
}

fn read_cpu_state() -> (Option<f64>, BTreeMap<String, String>) {
    let mut freqs = Vec::new();
    let mut governors = BTreeMap::new();
    let base = Path::new("/sys/devices/system/cpu");
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return (None, governors),
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_cpu_dir(&name) {
            continue;
        }
        let cpufreq = entry.path().join("cpufreq");
        if let Some(freq) = read_f64_file(cpufreq.join("scaling_cur_freq")) {
            freqs.push(freq);
        }
        if let Some(gov) = read_trim(cpufreq.join("scaling_governor")) {
            governors.insert(name, gov);
        }
    }

    let avg = if freqs.is_empty() {
        None
    } else {
        Some(freqs.iter().sum::<f64>() / freqs.len() as f64)
    };
    (avg, governors)
}

fn is_cpu_dir(name: &str) -> bool {
    if !name.starts_with("cpu") {
        return false;
    }
    name[3..].chars().all(|c| c.is_ascii_digit())
}

fn read_gpu_devfreq() -> (Option<u64>, Option<u64>) {
    let base = Path::new("/sys/class/devfreq");
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return (None, None),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let target = fs::read_link(&path)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let dev_name = read_trim(path.join("name")).unwrap_or_default();
        let haystack = format!("{} {} {}", file_name, target, dev_name).to_lowercase();
        if haystack.contains("gpu") || haystack.contains("gbus") || haystack.contains("57000000") {
            let cur = read_u64_file(path.join("cur_freq"));
            let max = read_u64_file(path.join("max_freq"));
            return (cur, max);
        }
    }
    (None, None)
}

fn read_ina3221_power_mw() -> Option<f64> {
    let base = Path::new("/sys/bus/i2c/drivers/ina3221x");
    if !base.exists() {
        return None;
    }
    let mut files = Vec::new();
    collect_power_files(base, 0, &mut files);
    let mut total_mw = 0.0;
    let mut count = 0;
    for path in files {
        if let Some(raw) = read_f64_file(path) {
            // Linux IIO power inputs are commonly microwatts. Treat values > 10000 as microwatts.
            let mw = if raw > 10000.0 { raw / 1000.0 } else { raw };
            total_mw += mw;
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(total_mw)
    }
}

fn collect_power_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 5 {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            collect_power_files(&path, depth + 1, out);
        } else if name.starts_with("in_power") && name.ends_with("_input") {
            out.push(path);
        }
    }
}

fn read_workload_hint(path: &str, stale_after_s: u64) -> WorkloadHint {
    let p = Path::new(path);
    let metadata = match fs::metadata(p) {
        Ok(m) => m,
        Err(_) => {
            return WorkloadHint {
                stale: true,
                ..WorkloadHint::default()
            }
        }
    };

    let stale = metadata
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .map(|age| age.as_secs() > stale_after_s)
        .unwrap_or(true);

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(_) => {
            return WorkloadHint {
                stale,
                ..WorkloadHint::default()
            }
        }
    };

    let mut kv = BTreeMap::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = if let Some(pos) = line.find('=') {
            (&line[..pos], &line[pos + 1..])
        } else if let Some(pos) = line.find(':') {
            (&line[..pos], &line[pos + 1..])
        } else {
            continue;
        };
        kv.insert(key.trim().to_string(), value.trim().to_string());
    }

    let fps = kv.get("fps").and_then(|v| v.parse::<f64>().ok());
    let busy = kv
        .get("busy")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false);
    let phase = kv.get("phase").cloned();
    let active = !stale && (busy || fps.unwrap_or(0.0) > 0.1);

    WorkloadHint {
        active,
        fps,
        phase,
        stale,
    }
}

fn decide(
    policy: &Policy,
    telemetry: &Telemetry,
    workload: &WorkloadHint,
    state: &mut DecisionState,
) -> Decision {
    let candidate = desired_decision(policy, telemetry, workload, state);
    hold_decision(policy, telemetry.ts_unix, candidate, state)
}

fn desired_decision(
    policy: &Policy,
    telemetry: &Telemetry,
    workload: &WorkloadHint,
    state: &mut DecisionState,
) -> Decision {
    if telemetry.max_temp_c.is_none() && policy.missing_thermal_is_hot {
        state.critical_cooldown_latched = false;
        state.thermal_guard_latched = true;
        return hot_decision(policy, "missing_thermal_telemetry");
    }

    if let Some(max_temp) = telemetry.max_temp_c {
        if max_temp >= policy.critical_temp_c {
            state.critical_cooldown_latched = true;
            state.thermal_guard_latched = true;
        } else if state.critical_cooldown_latched
            && max_temp <= policy.critical_temp_c - policy.critical_hysteresis_c
        {
            state.critical_cooldown_latched = false;
        }

        if state.critical_cooldown_latched {
            return hot_decision(policy, "critical_cooldown");
        }

        if max_temp >= policy.max_temp_c {
            state.thermal_guard_latched = true;
        } else if state.thermal_guard_latched
            && max_temp <= policy.max_temp_c - policy.thermal_hysteresis_c
        {
            state.thermal_guard_latched = false;
        }

        if state.thermal_guard_latched {
            return hot_decision(policy, "thermal_guard");
        }
    }

    if workload.active {
        let fps = workload.fps.unwrap_or(policy.min_fps);
        let reason = if fps < policy.min_fps {
            "active_below_fps_target"
        } else {
            "active"
        };
        return Decision {
            reason: reason.to_string(),
            nvpmodel_mode: policy.active_nvpmodel_mode,
            cpu_governor: policy.active_cpu_governor.clone(),
        };
    }
    Decision {
        reason: if workload.stale {
            "idle_or_no_workload_hint"
        } else {
            "idle"
        }
        .to_string(),
        nvpmodel_mode: policy.idle_nvpmodel_mode,
        cpu_governor: policy.idle_cpu_governor.clone(),
    }
}

fn hold_decision(
    policy: &Policy,
    now_unix: u64,
    candidate: Decision,
    state: &mut DecisionState,
) -> Decision {
    if let Some(last) = &state.last_decision {
        let elapsed = now_unix.saturating_sub(state.last_change_unix);
        let safety_escalation = safety_rank(&candidate.reason) > safety_rank(&last.reason);
        if candidate != *last && !safety_escalation && elapsed < policy.decision_hold_s {
            return last.clone();
        }
    }

    if state.last_decision.as_ref() != Some(&candidate) {
        state.last_change_unix = now_unix;
        state.last_decision = Some(candidate.clone());
    }
    candidate
}

fn hot_decision(policy: &Policy, reason: &str) -> Decision {
    Decision {
        reason: reason.to_string(),
        nvpmodel_mode: policy.hot_nvpmodel_mode,
        cpu_governor: policy.hot_cpu_governor.clone(),
    }
}

fn safety_rank(reason: &str) -> u8 {
    match reason {
        "critical_cooldown" | "missing_thermal_telemetry" => 3,
        "thermal_guard" => 2,
        _ => 1,
    }
}

fn apply_decision(
    policy: &Policy,
    decision: &Decision,
    dry_run: bool,
    last_nvpmodel_mode: &mut Option<u8>,
) -> Result<(), String> {
    validate_decision_allowed(policy, decision)?;
    set_cpu_governor(&decision.cpu_governor, dry_run)?;

    if *last_nvpmodel_mode != Some(decision.nvpmodel_mode) {
        set_nvpmodel(policy, decision.nvpmodel_mode, dry_run)?;
        *last_nvpmodel_mode = Some(decision.nvpmodel_mode);
    }
    Ok(())
}

fn validate_decision_allowed(policy: &Policy, decision: &Decision) -> Result<(), String> {
    if !policy.allowed_cpu_governors.is_empty()
        && !policy
            .allowed_cpu_governors
            .iter()
            .any(|governor| governor == &decision.cpu_governor)
    {
        return Err(format!(
            "cpu governor {} is not allowed by policy",
            decision.cpu_governor
        ));
    }
    if !policy.allowed_nvpmodel_modes.is_empty()
        && !policy
            .allowed_nvpmodel_modes
            .iter()
            .any(|mode| *mode == decision.nvpmodel_mode)
    {
        return Err(format!(
            "nvpmodel mode {} is not allowed by policy",
            decision.nvpmodel_mode
        ));
    }
    Ok(())
}

fn set_cpu_governor(governor: &str, dry_run: bool) -> Result<(), String> {
    let base = Path::new("/sys/devices/system/cpu");
    let entries = fs::read_dir(base).map_err(|e| format!("read cpu dir: {}", e))?;
    let mut attempted = 0;
    let mut failures = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_cpu_dir(&name) {
            continue;
        }
        let path = entry.path().join("cpufreq/scaling_governor");
        if !path.exists() {
            continue;
        }
        attempted += 1;
        if let Some(available) = read_trim(entry.path().join("cpufreq/scaling_available_governors"))
        {
            let listed = available.split_whitespace().any(|item| item == governor);
            if !listed {
                failures.push(format!(
                    "{}: governor {} is not in scaling_available_governors ({})",
                    path.display(),
                    governor,
                    available
                ));
                continue;
            }
        }
        if dry_run {
            println!(
                "thermagentd: dry-run write {} -> {}",
                path.display(),
                governor
            );
        } else if let Err(err) = fs::write(&path, format!("{}\n", governor)) {
            failures.push(format!("{}: {}", path.display(), err));
        }
    }

    if attempted == 0 {
        if dry_run {
            println!("thermagentd: dry-run no CPU cpufreq governor files found");
            return Ok(());
        }
        return Err("no CPU cpufreq governor files found".to_string());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn set_nvpmodel(policy: &Policy, mode: u8, dry_run: bool) -> Result<(), String> {
    if dry_run {
        println!(
            "thermagentd: dry-run exec {} -m {}",
            policy.nvpmodel_bin, mode
        );
        return Ok(());
    }

    let status = Command::new(&policy.nvpmodel_bin)
        .arg("-m")
        .arg(mode.to_string())
        .status()
        .map_err(|e| format!("exec {}: {}", policy.nvpmodel_bin, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} -m {} exited with {}",
            policy.nvpmodel_bin, mode, status
        ))
    }
}

fn write_state_file(
    path: &str,
    policy: &Policy,
    telemetry: &Telemetry,
    workload: &WorkloadHint,
    decision: &Decision,
    actions_failed: u64,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Ok(());
    }
    let body = render_state_json(policy, telemetry, workload, decision, actions_failed);
    let state_path = Path::new(path);
    if let Some(parent) = state_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create state dir {}: {}", parent.display(), err))?;
        }
    }
    let tmp_path = state_path.with_extension("json.tmp");
    fs::write(&tmp_path, body).map_err(|err| format!("write {}: {}", tmp_path.display(), err))?;
    fs::rename(&tmp_path, state_path).map_err(|err| {
        format!(
            "rename {} -> {}: {}",
            tmp_path.display(),
            state_path.display(),
            err
        )
    })
}

fn render_state_json(
    policy: &Policy,
    telemetry: &Telemetry,
    workload: &WorkloadHint,
    decision: &Decision,
    actions_failed: u64,
) -> String {
    format!(
        "{{\n  \"ts_unix\": {},\n  \"policy\": \"{}\",\n  \"reason\": \"{}\",\n  \"max_temp_c\": {},\n  \"workload_active\": {},\n  \"workload_fps\": {},\n  \"workload_phase\": {},\n  \"workload_stale\": {},\n  \"nvpmodel_mode\": {},\n  \"cpu_governor\": \"{}\",\n  \"actions_failed\": {}\n}}\n",
        telemetry.ts_unix,
        escape_json(&policy.name),
        escape_json(&decision.reason),
        json_opt_f64(telemetry.max_temp_c),
        workload.active,
        json_opt_f64(workload.fps),
        json_opt_string(workload.phase.as_deref()),
        workload.stale,
        decision.nvpmodel_mode,
        escape_json(&decision.cpu_governor),
        actions_failed
    )
}

fn json_opt_f64(value: Option<f64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_string(value: Option<&str>) -> String {
    value
        .map(|v| format!("\"{}\"", escape_json(v)))
        .unwrap_or_else(|| "null".to_string())
}

fn serve_metrics(addr: String, shared: Arc<Mutex<SharedMetrics>>) -> Result<(), String> {
    let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {}: {}", addr, e))?;
    println!(
        "thermagentd: prometheus metrics listening on http://{}/metrics",
        addr
    );
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let metrics = shared.lock().map(|g| g.clone()).unwrap_or_default();
                if let Err(err) = respond_metrics(&mut s, &metrics) {
                    eprintln!("thermagentd: metrics response failed: {}", err);
                }
            }
            Err(err) => eprintln!("thermagentd: metrics accept failed: {}", err),
        }
    }
    Ok(())
}

fn respond_metrics(stream: &mut TcpStream, m: &SharedMetrics) -> Result<(), String> {
    let mut buf = [0_u8; 512];
    let _ = stream.read(&mut buf);
    let body = render_prometheus(m);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
        body.len(), body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| e.to_string())
}

fn render_prometheus(m: &SharedMetrics) -> String {
    let mut out = String::new();
    out.push_str("# HELP thermagent_info Static daemon info.\n");
    out.push_str("# TYPE thermagent_info gauge\n");
    out.push_str(&format!(
        "thermagent_info{{policy=\"{}\",dry_run=\"{}\"}} 1\n",
        escape_label(&m.policy_name),
        m.dry_run
    ));
    out.push_str("# HELP thermagent_timestamp_seconds Last telemetry sample timestamp.\n");
    out.push_str("# TYPE thermagent_timestamp_seconds gauge\n");
    out.push_str(&format!("thermagent_timestamp_seconds {}\n", m.ts_unix));
    push_opt(
        &mut out,
        "thermagent_temperature_celsius",
        "Max thermal-zone temperature.",
        m.max_temp_c,
    );
    if !m.thermal_zones.is_empty() {
        out.push_str("# HELP thermagent_thermal_zone_temperature_celsius Thermal-zone temperature by zone.\n");
        out.push_str("# TYPE thermagent_thermal_zone_temperature_celsius gauge\n");
        for zone in &m.thermal_zones {
            out.push_str(&format!(
                "thermagent_thermal_zone_temperature_celsius{{zone=\"{}\"}} {}\n",
                escape_label(&zone.name),
                zone.temp_c
            ));
        }
    }
    push_opt(
        &mut out,
        "thermagent_cpu_freq_khz",
        "Average CPU frequency from cpufreq.",
        m.avg_cpu_freq_khz,
    );
    if !m.cpu_governors.is_empty() {
        out.push_str(
            "# HELP thermagent_cpu_scaling_governor_info Current CPU cpufreq governor labels.\n",
        );
        out.push_str("# TYPE thermagent_cpu_scaling_governor_info gauge\n");
        for (cpu, governor) in &m.cpu_governors {
            out.push_str(&format!(
                "thermagent_cpu_scaling_governor_info{{cpu=\"{}\",governor=\"{}\"}} 1\n",
                escape_label(cpu),
                escape_label(governor)
            ));
        }
    }
    push_opt(
        &mut out,
        "thermagent_gpu_cur_freq_hz",
        "GPU-like devfreq current frequency.",
        m.gpu_cur_freq_hz.map(|v| v as f64),
    );
    push_opt(
        &mut out,
        "thermagent_gpu_max_freq_hz",
        "GPU-like devfreq max frequency.",
        m.gpu_max_freq_hz.map(|v| v as f64),
    );
    push_opt(
        &mut out,
        "thermagent_power_mw_sum",
        "Sum of discovered INA3221 power input files.",
        m.power_mw_sum,
    );
    out.push_str("# HELP thermagent_workload_active Workload hint active flag.\n");
    out.push_str("# TYPE thermagent_workload_active gauge\n");
    out.push_str(&format!(
        "thermagent_workload_active {}\n",
        if m.workload_active { 1 } else { 0 }
    ));
    push_opt(
        &mut out,
        "thermagent_workload_fps",
        "Workload hint FPS.",
        m.workload_fps,
    );
    out.push_str("# HELP thermagent_workload_stale Workload hint staleness flag.\n");
    out.push_str("# TYPE thermagent_workload_stale gauge\n");
    out.push_str(&format!(
        "thermagent_workload_stale {}\n",
        if m.workload_stale { 1 } else { 0 }
    ));
    if let Some(phase) = &m.workload_phase {
        out.push_str("# HELP thermagent_workload_phase_info Workload phase label from hint.\n");
        out.push_str("# TYPE thermagent_workload_phase_info gauge\n");
        out.push_str(&format!(
            "thermagent_workload_phase_info{{phase=\"{}\"}} 1\n",
            escape_label(phase)
        ));
    }
    if let Some(mode) = m.nvpmodel_mode {
        out.push_str("# HELP thermagent_nvpmodel_mode Desired Jetson nvpmodel mode.\n");
        out.push_str("# TYPE thermagent_nvpmodel_mode gauge\n");
        out.push_str(&format!(
            "thermagent_nvpmodel_mode{{reason=\"{}\"}} {}\n",
            escape_label(&m.reason),
            mode
        ));
    }
    if let Some(governor) = &m.cpu_governor {
        out.push_str("# HELP thermagent_cpu_governor_info Desired CPU governor label.\n");
        out.push_str("# TYPE thermagent_cpu_governor_info gauge\n");
        out.push_str(&format!(
            "thermagent_cpu_governor_info{{governor=\"{}\",reason=\"{}\"}} 1\n",
            escape_label(governor),
            escape_label(&m.reason)
        ));
    }
    out.push_str("# HELP thermagent_actions_failed_total Failed action attempts.\n");
    out.push_str("# TYPE thermagent_actions_failed_total counter\n");
    out.push_str(&format!(
        "thermagent_actions_failed_total {}\n",
        m.actions_failed
    ));
    out
}

fn push_opt(out: &mut String, name: &str, help: &str, value: Option<f64>) {
    if let Some(v) = value {
        out.push_str(&format!("# HELP {} {}\n", name, help));
        out.push_str(&format!("# TYPE {} gauge\n", name));
        out.push_str(&format!("{} {}\n", name, v));
    }
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn escape_json(value: &str) -> String {
    escape_label(value)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn read_trim<P: AsRef<Path>>(path: P) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_f64_file<P: AsRef<Path>>(path: P) -> Option<f64> {
    read_trim(path).and_then(|s| s.parse::<f64>().ok())
}

fn read_u64_file<P: AsRef<Path>>(path: P) -> Option<u64> {
    read_trim(path).and_then(|s| s.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_policy_text() -> &'static str {
        r#"
name: jetson-nano-lite-safe-vision
sample_interval_ms: 1000
max_temp_c: 72.0
critical_temp_c: 78.0
min_fps: 12.0
workload_stale_after_s: 15
idle_nvpmodel_mode: 1
active_nvpmodel_mode: 0
hot_nvpmodel_mode: 1
idle_cpu_governor: schedutil
active_cpu_governor: schedutil
hot_cpu_governor: powersave
nvpmodel_bin: /usr/sbin/nvpmodel
workload_file: /run/thermagent/workload.metrics
prometheus_listen: 127.0.0.1:9920
state_file: /var/lib/thermagent/state.json
allowed_cpu_governors: schedutil,powersave
allowed_nvpmodel_modes: 0,1
"#
    }

    fn write_temp_policy(name: &str, body: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "thermagentd-{}-{}.policy",
            name,
            std::process::id()
        ));
        fs::write(&path, body).expect("write temp policy");
        path
    }

    #[test]
    fn read_policy_rejects_missing_required_keys() {
        let path = write_temp_policy("missing", "name: incomplete\n");
        let err = read_policy(path.to_str().expect("utf-8 path")).expect_err("policy should fail");
        let _ = fs::remove_file(path);
        assert!(err.contains("missing policy keys"));
        assert!(err.contains("sample_interval_ms"));
    }

    #[test]
    fn read_policy_accepts_complete_policy() {
        let path = write_temp_policy("complete", sample_policy_text());
        let policy = read_policy(path.to_str().expect("utf-8 path")).expect("policy should parse");
        let _ = fs::remove_file(path);
        assert_eq!(policy.name, "jetson-nano-lite-safe-vision");
        assert_eq!(policy.hot_cpu_governor, "powersave");
        assert_eq!(policy.state_file, "/var/lib/thermagent/state.json");
        assert_eq!(policy.allowed_cpu_governors, vec!["schedutil", "powersave"]);
        assert_eq!(policy.allowed_nvpmodel_modes, vec![0, 1]);
    }

    #[test]
    fn decide_prioritizes_critical_temperature_over_workload() {
        let policy = Policy::default();
        let telemetry = Telemetry {
            ts_unix: 10,
            max_temp_c: Some(policy.critical_temp_c),
            ..Telemetry::default()
        };
        let workload = WorkloadHint {
            active: true,
            fps: Some(policy.min_fps + 30.0),
            ..WorkloadHint::default()
        };

        let mut state = DecisionState::default();
        let decision = decide(&policy, &telemetry, &workload, &mut state);

        assert_eq!(decision.reason, "critical_cooldown");
        assert_eq!(decision.nvpmodel_mode, policy.hot_nvpmodel_mode);
        assert_eq!(decision.cpu_governor, policy.hot_cpu_governor);
    }

    #[test]
    fn decide_keeps_thermal_guard_until_hysteresis_clears() {
        let policy = Policy::default();
        let mut state = DecisionState::default();
        let workload = WorkloadHint {
            active: true,
            fps: Some(policy.min_fps + 1.0),
            ..WorkloadHint::default()
        };

        let hot = decide(
            &policy,
            &Telemetry {
                ts_unix: 10,
                max_temp_c: Some(policy.max_temp_c + 0.5),
                ..Telemetry::default()
            },
            &workload,
            &mut state,
        );
        let still_hot = decide(
            &policy,
            &Telemetry {
                ts_unix: 20,
                max_temp_c: Some(policy.max_temp_c - 1.0),
                ..Telemetry::default()
            },
            &workload,
            &mut state,
        );
        let clear = decide(
            &policy,
            &Telemetry {
                ts_unix: 30,
                max_temp_c: Some(policy.max_temp_c - policy.thermal_hysteresis_c),
                ..Telemetry::default()
            },
            &workload,
            &mut state,
        );

        assert_eq!(hot.reason, "thermal_guard");
        assert_eq!(still_hot.reason, "thermal_guard");
        assert_eq!(clear.reason, "active");
    }

    #[test]
    fn decide_uses_safe_mode_when_thermal_telemetry_is_missing() {
        let policy = Policy::default();
        let mut state = DecisionState::default();
        let decision = decide(
            &policy,
            &Telemetry {
                ts_unix: 10,
                max_temp_c: None,
                ..Telemetry::default()
            },
            &WorkloadHint::default(),
            &mut state,
        );

        assert_eq!(decision.reason, "missing_thermal_telemetry");
        assert_eq!(decision.nvpmodel_mode, policy.hot_nvpmodel_mode);
    }

    #[test]
    fn decide_holds_non_safety_transitions_briefly() {
        let policy = Policy {
            decision_hold_s: 10,
            ..Policy::default()
        };
        let mut state = DecisionState::default();
        let active = WorkloadHint {
            active: true,
            fps: Some(policy.min_fps + 1.0),
            ..WorkloadHint::default()
        };
        let idle = WorkloadHint::default();

        let first = decide(
            &policy,
            &Telemetry {
                ts_unix: 10,
                max_temp_c: Some(40.0),
                ..Telemetry::default()
            },
            &active,
            &mut state,
        );
        let held = decide(
            &policy,
            &Telemetry {
                ts_unix: 15,
                max_temp_c: Some(40.0),
                ..Telemetry::default()
            },
            &idle,
            &mut state,
        );
        let released = decide(
            &policy,
            &Telemetry {
                ts_unix: 25,
                max_temp_c: Some(40.0),
                ..Telemetry::default()
            },
            &idle,
            &mut state,
        );

        assert_eq!(first.reason, "active");
        assert_eq!(held.reason, "active");
        assert_eq!(released.reason, "idle");
    }

    #[test]
    fn validate_decision_allowed_rejects_unlisted_actions() {
        let policy = Policy {
            allowed_cpu_governors: vec!["powersave".to_string()],
            allowed_nvpmodel_modes: vec![1],
            ..Policy::default()
        };
        let decision = Decision {
            reason: "active".to_string(),
            nvpmodel_mode: 0,
            cpu_governor: "schedutil".to_string(),
        };

        let err = validate_decision_allowed(&policy, &decision).expect_err("decision should fail");

        assert!(err.contains("cpu governor schedutil is not allowed"));
    }

    #[test]
    fn render_state_json_includes_debuggable_last_state() {
        let policy = Policy::default();
        let telemetry = Telemetry {
            ts_unix: 123,
            max_temp_c: Some(61.5),
            ..Telemetry::default()
        };
        let workload = WorkloadHint {
            active: true,
            fps: Some(12.5),
            phase: Some("vision".to_string()),
            stale: false,
        };
        let decision = Decision {
            reason: "active".to_string(),
            nvpmodel_mode: 0,
            cpu_governor: "schedutil".to_string(),
        };

        let body = render_state_json(&policy, &telemetry, &workload, &decision, 2);

        assert!(body.contains("\"ts_unix\": 123"));
        assert!(body.contains("\"reason\": \"active\""));
        assert!(body.contains("\"workload_phase\": \"vision\""));
        assert!(body.contains("\"actions_failed\": 2"));
    }

    #[test]
    fn prometheus_metrics_include_collected_telemetry() {
        let mut cpu_governors = BTreeMap::new();
        cpu_governors.insert("cpu0".to_string(), "schedutil".to_string());
        let metrics = SharedMetrics {
            policy_name: "jetson".to_string(),
            thermal_zones: vec![ThermalZone {
                name: "CPU-therm".to_string(),
                temp_c: 64.5,
            }],
            cpu_governors,
            workload_active: true,
            workload_phase: Some("vision".to_string()),
            workload_stale: false,
            ..SharedMetrics::default()
        };

        let body = render_prometheus(&metrics);

        assert!(
            body.contains("thermagent_thermal_zone_temperature_celsius{zone=\"CPU-therm\"} 64.5")
        );
        assert!(body.contains(
            "thermagent_cpu_scaling_governor_info{cpu=\"cpu0\",governor=\"schedutil\"} 1"
        ));
        assert!(body.contains("thermagent_workload_phase_info{phase=\"vision\"} 1"));
        assert!(body.contains("thermagent_workload_stale 0"));
    }
}
