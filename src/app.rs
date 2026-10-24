use crate::curve::{self, fmt_supply_short, fmt_units, fmt_units_short};
use crate::network::Network;
use crate::rpc::{Rpc, DEFAULT_MEMO, MAX_MEMO_BYTES, MIN_MEMO_BYTES};
use crate::wallet::WalletKind;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

/// Where users download the desktop miner; None until it is published
const MINER_URL: Option<&str> = None;

const LAMPORTS_PER_XNT: u64 = 1_000_000_000;

#[derive(Clone, Copy)]
struct AppState {
    network: RwSignal<Network>,
    wallet: RwSignal<Option<(WalletKind, String)>>,
    supply: RwSignal<Option<u64>>,
    /// (XNT lamports, MEMO units) of the connected wallet
    balances: RwSignal<Option<(u64, u64)>>,
    show_wallets: RwSignal<bool>,
}

impl AppState {
    fn refresh(&self) {
        let state = *self;
        spawn_local(async move {
            let network = state.network.get_untracked();
            let rpc = Rpc::new(network);
            match rpc.token_supply().await {
                Ok(s) if state.network.get_untracked() == network => state.supply.set(Some(s)),
                Ok(_) => {}
                Err(e) => log::warn!("supply: {}", e),
            }
            if let Some((_, owner)) = state.wallet.get_untracked() {
                if let (Ok(xnt), Ok(memo)) = (rpc.xnt_balance(&owner).await, rpc.memo_balance(&owner).await) {
                    if state.network.get_untracked() == network {
                        state.balances.set(Some((xnt, memo)));
                    }
                }
            }
        });
    }
}

fn short_address(a: &str) -> String {
    if a.len() > 10 { format!("{}…{}", &a[..4], &a[a.len() - 4..]) } else { a.to_string() }
}

fn fmt_xnt(lamports: u64) -> String {
    let milli = lamports / (LAMPORTS_PER_XNT / 10_000);
    format!("{}.{:04}", curve::group(milli / 10_000), milli % 10_000)
}

#[component]
pub fn App() -> impl IntoView {
    let state = AppState {
        network: create_rw_signal(Network::load()),
        wallet: create_rw_signal(None),
        supply: create_rw_signal(None),
        balances: create_rw_signal(None),
        show_wallets: create_rw_signal(false),
    };
    provide_context(state);

    // Reload on network or wallet change, and every minute
    create_effect(move |_| {
        state.network.track();
        state.wallet.track();
        state.supply.set(None);
        state.balances.set(None);
        state.refresh();
    });
    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(60_000).await;
            state.refresh();
        }
    });

    view! {
        <div class="shell">
            <aside class="sidebar">
                <div class="brand">
                    <img src="/public/memo-logo.png" alt=""/>
                    <div class="brand-text"><span>"MEMO"</span><small>"Engraves Memories Onchain"</small></div>
                </div>
                <nav>
                    <a class="nav-item active" href="/"><span class="nav-icon">"◎"</span>"MEMO Token"</a>
                </nav>
            </aside>
            <div class="main">
                <header class="topbar">
                    <div class="brand brand-mobile"><img src="/public/memo-logo.png" alt=""/><span>"MEMO"</span></div>
                    <div class="topbar-right">
                        <NetworkSelect/>
                        <WalletButton/>
                    </div>
                </header>
                <main class="page"><TokenPage/></main>
            </div>
            <Show when=move || state.show_wallets.get()><WalletModal/></Show>
        </div>
    }
}

#[component]
fn NetworkSelect() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <select class="pill network-select" aria-label="Network"
            on:change=move |ev| {
                let name = event_target_value(&ev);
                if let Some(n) = Network::ALL.into_iter().find(|n| n.name() == name) {
                    n.save();
                    state.network.set(n);
                }
            }
        >
            {Network::ALL.into_iter().map(|n| view! {
                <option value=n.name() selected=move || state.network.get() == n>{n.name()}</option>
            }).collect_view()}
        </select>
    }
}

#[component]
fn WalletButton() -> impl IntoView {
    let state = expect_context::<AppState>();
    let menu = create_rw_signal(false);
    view! {
        {move || match state.wallet.get() {
            None => view! {
                <button class="btn-primary btn-small" on:click=move |_| state.show_wallets.set(true)>"Connect wallet"</button>
            }.into_view(),
            Some((kind, address)) => view! {
                <div class="wallet-menu-anchor">
                    <button class="pill" on:click=move |_| menu.update(|m| *m = !*m)>
                        <span class="dot"></span>{short_address(&address)}
                    </button>
                    <Show when=move || menu.get()>
                        <div class="wallet-menu">
                            <div class="wallet-menu-row"><span>"Wallet"</span><span>{kind.name()}</span></div>
                            <div class="wallet-menu-row">
                                <span>"XNT"</span>
                                <span>{move || state.balances.get().map(|(x, _)| fmt_xnt(x)).unwrap_or("…".into())}</span>
                            </div>
                            <div class="wallet-menu-row">
                                <span>"MEMO"</span>
                                <span>{move || state.balances.get().map(|(_, m)| fmt_units_short(m, 2)).unwrap_or("…".into())}</span>
                            </div>
                            <button class="btn-secondary" on:click=move |_| {
                                menu.set(false);
                                state.wallet.set(None);
                                spawn_local(kind.disconnect());
                            }>"Disconnect"</button>
                        </div>
                    </Show>
                </div>
            }.into_view(),
        }}
    }
}

#[component]
fn WalletModal() -> impl IntoView {
    let state = expect_context::<AppState>();
    let error = create_rw_signal(None::<String>);
    let close = move |_| state.show_wallets.set(false);
    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal" on:click=|ev| ev.stop_propagation()>
                <div class="modal-head">
                    <h2>"Connect a wallet"</h2>
                    <button class="icon-btn" on:click=close aria-label="Close">"✕"</button>
                </div>
                {WalletKind::ALL.into_iter().map(|kind| {
                    let installed = kind.is_installed();
                    view! {
                        <div class="wallet-option">
                            <span>{kind.name()}</span>
                            {if installed {
                                view! {
                                    <button class="btn-secondary" on:click=move |_| {
                                        error.set(None);
                                        spawn_local(async move {
                                            match kind.connect().await {
                                                Ok(address) => {
                                                    state.wallet.set(Some((kind, address)));
                                                    state.show_wallets.set(false);
                                                }
                                                Err(e) => error.set(Some(e)),
                                            }
                                        });
                                    }>"Connect"</button>
                                }.into_view()
                            } else {
                                view! {
                                    <a class="btn-secondary" href=kind.install_url() target="_blank" rel="noopener">"Install ↗"</a>
                                }.into_view()
                            }}
                        </div>
                    }
                }).collect_view()}
                <Show when=move || error.get().is_some()>
                    <p class="msg msg-error">{move || error.get()}</p>
                </Show>
                <p class="modal-note">"Your keys stay in your wallet. MEMO only asks it to sign."</p>
            </div>
        </div>
    }
}

#[component]
fn TokenPage() -> impl IntoView {
    view! {
        <section class="hero">
            <img class="hero-logo" src="/public/memo-logo.png" alt="MEMO"/>
            <h1 class="motto">
                <span><b>"M"</b>"EMO"</span>" "<span><b>"E"</b>"ngraves"</span>" "
                <span><b>"M"</b>"emories"</span>" "<span><b>"O"</b>"nchain"</span>
            </h1>
            <p>"Write a memo on X1 and it is engraved onchain for good. Every mint carries one, and pays you MEMO for it. The reward steps down as the supply grows, until the 10T cap."</p>
        </section>
        <div class="home-grid">
            <SupplyPanel/>
            <div class="side-col">
                <MintCard/>
                <TpmPanel/>
            </div>
        </div>
    }
}

/// Staircase of the whole curve: equal-width stages (9 equal steps inside each),
/// reward on a log scale; the mined part is filled and the marker shows the current supply
fn curve_svg(supply: u64) -> String {
    const W: f64 = 700.0;
    const H: f64 = 220.0;
    const PAD: f64 = 12.0;
    let stages = curve::stages();
    let stage_w = W / stages.len() as f64;
    let y = |units: u64| PAD + (6.0 - (units.max(1) as f64).log10()) / 6.0 * (H - 2.0 * PAD);

    let mut line = String::new();
    for (i, stage) in stages.iter().enumerate() {
        let step_w = stage_w / stage.steps.len() as f64;
        for (j, step) in stage.steps.iter().enumerate() {
            let x0 = i as f64 * stage_w + j as f64 * step_w;
            let cmd = if line.is_empty() { "M" } else { "L" };
            line.push_str(&format!("{}{:.1},{:.1} H{:.1} ", cmd, x0, y(step.reward), x0 + step_w));
        }
    }
    let area = format!("{}V{:.1} H0 Z", line, H);
    let mined = (curve::position(supply) * W).clamp(0.0, W);
    let mx = mined.clamp(2.0, W - 2.0);
    let my = y(curve::reward_at(supply));
    let grid: String = (1..stages.len())
        .map(|i| {
            let x = i as f64 * stage_w;
            format!(r#"<line class="c-grid" x1="{x:.1}" y1="0" x2="{x:.1}" y2="{H}"/>"#)
        })
        .collect();
    format!(
        r#"<svg class="curve-svg" viewBox="0 0 {W} {H}" role="img" aria-label="Mint reward by total supply">
<defs>
<linearGradient id="c-fade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="currentColor" stop-opacity="0.35"/><stop offset="1" stop-color="currentColor" stop-opacity="0"/></linearGradient>
<clipPath id="c-mined"><rect x="0" y="0" width="{mined:.1}" height="{H}"/></clipPath>
</defs>
{grid}
<path class="c-area" d="{area}"/>
<path class="c-area-mined" d="{area}" fill="url(#c-fade)" clip-path="url(#c-mined)"/>
<path class="c-line" d="{line}"/>
<path class="c-line-mined" d="{line}" clip-path="url(#c-mined)"/>
<line class="c-marker" x1="{mx:.1}" y1="0" x2="{mx:.1}" y2="{H}"/>
<circle class="c-dot" cx="{mx:.1}" cy="{my:.1}" r="5"/>
</svg>"#
    )
}

#[component]
fn SupplyPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    // Which stage's steps the table shows; None = the current stage (kept across supply refreshes)
    let expanded = create_rw_signal(None::<usize>);
    view! {
        <section class="panel supply-panel">
            <div class="panel-label">"MEMO supply"</div>
            {move || match state.supply.get() {
                None => view! { <div class="supply-loading">"Loading…"</div> }.into_view(),
                Some(supply) => {
                    let reward = curve::reward_at(supply);
                    let (stage_idx, _) = curve::locate(supply);
                    let next = curve::next_reduction(supply);
                    view! {
                        <div class="supply-figure">
                            <span class="big-number">{fmt_units_short(supply, 0)}</span>
                            <span class="unit">"MEMO"</span>
                        </div>
                        <div class="supply-sub">
                            {match next {
                                Some((at, after)) => view! {
                                    <span class="accent">{format!("{} MEMO", fmt_units(reward))}</span>
                                    {format!(" per mint now · {} after {} ({} MEMO to go)",
                                        fmt_units(after), fmt_supply_short(at), fmt_units_short(at - supply, 0))}
                                }.into_view(),
                                None => format!("{} MEMO per mint · final step", fmt_units(reward)).into_view(),
                            }}
                        </div>
                        <div class="curve-chart" inner_html=curve_svg(supply)></div>
                        <div class="stage-row">
                            {curve::stages().iter().enumerate().map(|(i, s)| {
                                // Two parts, so narrow screens can put them on separate lines
                                let (from, to) = if i == 0 { ("≤".to_string(), fmt_supply_short(s.end)) }
                                                 else { (format!("{}–", fmt_supply_short(s.start)), fmt_supply_short(s.end)) };
                                // Reward at both ends of the stage, on two lines so long decimals fit
                                let first = s.steps[0].reward;
                                let last = s.steps[s.steps.len() - 1].reward;
                                let (r_from, r_to) = if first == last { (fmt_units(first), String::new()) }
                                                     else { (fmt_units(first), format!("→ {}", fmt_units(last))) };
                                view! {
                                    <div class="stage" class:current=i == stage_idx>
                                        <span class="stage-range"><span>{from}</span><span>{to}</span></span>
                                        <span class="stage-reward"><span>{r_from}</span><span>{r_to}</span></span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                        <p class="panel-note">"Reward per mint, log scale."</p>
                        <EmissionTable supply=supply expanded=expanded/>
                    }.into_view()
                }
            }}
        </section>
    }
}

/// The emission rules as a table: one row per stage; a click on a stage lists its 9 steps
#[component]
fn EmissionTable(supply: u64, expanded: RwSignal<Option<usize>>) -> impl IntoView {
    let stages = curve::stages();
    let (now_stage, now_step) = curve::locate(supply);
    // Show the current stage's steps, or the first reduction stage while supply is still in the flat start
    let default_open = now_stage.max(1);
    let open = move || expanded.get().unwrap_or(default_open);

    let rows = stages.into_iter().enumerate().map(|(i, stage)| {
        let first = stage.steps[0].reward;
        let last = stage.steps[stage.steps.len() - 1].reward;
        let range = format!("{} – {}", if i == 0 { "0".to_string() } else { fmt_supply_short(stage.start) }, fmt_supply_short(stage.end));
        let reward = if i == 0 { fmt_units(first) } else { format!("{} → {}", fmt_units(first), fmt_units(last)) };
        let step = if i == 0 {
            "fixed".to_string()
        } else {
            format!("−{} every {}", fmt_units(first / 9), fmt_supply_short(stage.start))
        };
        let state = if i < now_stage { "past" } else if i == now_stage { "now" } else { "future" };
        let steps = stage.steps.clone();
        view! {
            <tr class=format!("stage-tr {}", state) class:open=move || open() == i
                on:click=move |_| expanded.set(Some(if open() == i { usize::MAX } else { i }))
            >
                <td class="col-stage">{i}</td>
                <td>{range}{(i == now_stage).then(|| view! { <span class="now-badge">"now"</span> })}</td>
                <td class="mono">{reward}</td>
                <td class="col-step">{step}</td>
                <td class="col-toggle">{move || if open() == i { "−" } else { "+" }}</td>
            </tr>
            <Show when=move || open() == i>
                <tr class="steps-tr">
                    <td colspan="5">
                        <div class="steps-grid">
                            {steps.iter().enumerate().map(|(j, s)| {
                                let cls = if i < now_stage || (i == now_stage && j < now_step) { "past" }
                                          else if i == now_stage && j == now_step { "now" } else { "future" };
                                let from = if i == 0 { "0".to_string() } else { fmt_supply_short(s.start) };
                                view! {
                                    <div class=format!("step-cell {}", cls)>
                                        <span class="step-range">{format!("{} – {}", from, fmt_supply_short(s.end))}</span>
                                        <span class="step-reward mono">{fmt_units(s.reward)}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </td>
                </tr>
            </Show>
        }
    }).collect_view();

    view! {
        <div class="emission">
            <h3 class="emission-title">"How the reward changes"</h3>
            <ul class="emission-rules">
                <li>"Up to 10M supply, every mint pays 1 MEMO."</li>
                <li>"After that, each tenfold range of supply is a stage. A stage has 9 equal steps, and the reward drops by the same amount at every step, ending at a tenth of where the stage began."</li>
                <li>"A step changes once the supply passes the round number: a mint at exactly 20M still pays 0.9."</li>
                <li>"Minting stops at 10T. Every number here is computed exactly as the memo-mint contract does."</li>
            </ul>
            <table class="emission-table">
                <thead>
                    <tr><th>"Stage"</th><th>"Supply"</th><th>"MEMO per mint"</th><th class="col-step">"Step"</th><th></th></tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
            <p class="panel-note">"Click a stage to see its steps."</p>
        </div>
    }
}

const TPM_MINUTES: i64 = 30;

/// Local "HH:MM" for a unix time
fn hhmm(unix: i64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(unix as f64 * 1000.0));
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

/// Bars for the last TPM_MINUTES complete minutes plus the current one (drawn faded: still filling)
fn tpm_svg(start: i64, counts: &[u32]) -> String {
    const W: f64 = 360.0;
    const H: f64 = 120.0;
    let max = counts.iter().copied().max().unwrap_or(0).max(1) as f64;
    let slot = W / counts.len() as f64;
    let bars: String = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let h = (c as f64 / max * (H - 4.0)).max(if c > 0 { 2.0 } else { 0.0 });
            let class = if i + 1 == counts.len() { "t-bar partial" } else { "t-bar" };
            let t = start + i as i64 * 60;
            format!(
                r#"<rect class="{class}" x="{:.1}" y="{:.1}" width="{:.1}" height="{h:.1}" rx="1.5"><title>{} · {} tx</title></rect>"#,
                i as f64 * slot + 1.0,
                H - h,
                slot - 2.0,
                hhmm(t),
                c
            )
        })
        .collect();
    format!(
        r#"<svg class="tpm-svg" viewBox="0 0 {W} {H}" role="img" aria-label="Transactions per minute">
<line class="t-grid" x1="0" y1="{:.1}" x2="{W}" y2="{:.1}"/><line class="t-grid" x1="0" y1="{H}" x2="{W}" y2="{H}"/>
{bars}</svg>"#,
        H / 2.0,
        H / 2.0
    )
}

/// Mints per minute, read from recent memo-mint signatures over RPC
#[component]
fn TpmPanel() -> impl IntoView {
    let state = expect_context::<AppState>();
    // (start of the first minute, counts per minute; the last minute is the current one)
    let data = create_rw_signal(None::<(i64, Vec<u32>)>);
    let failed = create_rw_signal(false);

    let load = move || {
        spawn_local(async move {
            let network = state.network.get_untracked();
            let now = (js_sys::Date::now() / 1000.0) as i64;
            let current = now / 60 * 60;
            let start = current - TPM_MINUTES * 60;
            match Rpc::new(network).recent_tx_times(start, 10).await {
                Ok(times) if state.network.get_untracked() == network => {
                    let mut counts = vec![0u32; TPM_MINUTES as usize + 1];
                    for t in times.into_iter().filter(|&t| t >= start) {
                        if let Some(c) = counts.get_mut(((t - start) / 60) as usize) {
                            *c += 1;
                        }
                    }
                    failed.set(false);
                    data.set(Some((start, counts)));
                }
                Ok(_) => {}
                Err(e) => {
                    log::warn!("tpm: {}", e);
                    failed.set(true);
                }
            }
        });
    };
    create_effect(move |_| {
        state.network.track();
        data.set(None);
        load();
    });
    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(60_000).await;
            if data.try_get_untracked().is_none() {
                break; // the panel is gone
            }
            load();
        }
    });

    view! {
        <section class="panel tpm-panel">
            <div class="panel-label">"Transactions per minute"</div>
            {move || match data.get() {
                None if failed.get() => view! { <div class="tpm-empty">"Could not load recent transactions."</div> }.into_view(),
                None => view! { <div class="tpm-empty">"Loading…"</div> }.into_view(),
                Some((start, counts)) => {
                    let complete = &counts[..counts.len() - 1];
                    let recent = &complete[complete.len().saturating_sub(5)..];
                    let avg = recent.iter().sum::<u32>() as f64 / recent.len().max(1) as f64;
                    let total: u32 = complete.iter().sum();
                    let end = start + TPM_MINUTES * 60;
                    view! {
                        <div class="supply-figure">
                            <span class="big-number">{format!("{:.0}", avg)}</span>
                            <span class="unit">"TPM"</span>
                        </div>
                        <div class="supply-sub">
                            {format!("average of the last 5 minutes · {} in the last {} minutes", curve::group(total as u64), TPM_MINUTES)}
                        </div>
                        <div class="tpm-chart" inner_html=tpm_svg(start, &counts)></div>
                        <div class="tpm-axis">
                            <span>{hhmm(start)}</span>
                            <span>{hhmm(start + TPM_MINUTES * 30)}</span>
                            <span>{hhmm(end)}</span>
                        </div>
                        <p class="panel-note">"Mints of MEMO, counted from recent memo-mint transactions. Updates every minute."</p>
                    }.into_view()
                }
            }}
        </section>
    }
}

#[derive(Clone, PartialEq)]
enum MintStatus {
    Idle,
    Working(&'static str),
    Done { signature: String, reward: u64, network: Network },
    Failed(String),
}

#[component]
fn MintCard() -> impl IntoView {
    let state = expect_context::<AppState>();
    let memo = create_rw_signal(DEFAULT_MEMO.to_string());
    let status = create_rw_signal(MintStatus::Idle);
    let memo_len = move || memo.with(|m| m.len());
    let memo_ok = move || (MIN_MEMO_BYTES..=MAX_MEMO_BYTES).contains(&memo_len());
    let busy = move || matches!(status.get(), MintStatus::Working(_));
    let low_xnt = move || state.balances.get().is_some_and(|(xnt, _)| xnt < LAMPORTS_PER_XNT / 1_000);

    let mint = move |_| {
        let Some((kind, owner)) = state.wallet.get_untracked() else {
            state.show_wallets.set(true);
            return;
        };
        let network = state.network.get_untracked();
        let memo = memo.get_untracked();
        spawn_local(async move {
            let rpc = Rpc::new(network);
            status.set(MintStatus::Working("Preparing…"));
            let result = async {
                let supply = rpc.token_supply().await?;
                let tx = rpc.build_mint_transaction(&owner, &memo).await?;
                status.set(MintStatus::Working("Confirm in your wallet…"));
                let tx_b64 = base64::encode(bincode::serialize(&tx).map_err(|e| e.to_string())?);
                let signed = kind.sign(&tx_b64).await?;
                status.set(MintStatus::Working("Sending…"));
                let signature = rpc.send(&signed).await?;
                status.set(MintStatus::Working("Confirming…"));
                rpc.confirm(&signature).await?;
                Ok::<_, String>(MintStatus::Done { signature, reward: curve::reward_at(supply), network })
            }
            .await;
            status.set(result.unwrap_or_else(MintStatus::Failed));
            state.refresh();
        });
    };

    view! {
        <section class="panel mint-card">
            <div class="card-tabs"><span class="tab active">"Mint"</span></div>
            <p class="card-caption">"Your memo is written onchain with every mint."</p>

            <label class="field">
                <span class="field-label">"Your memo"</span>
                <textarea class="memo-input" rows="3" spellcheck="false"
                    prop:value=move || memo.get()
                    on:input=move |ev| memo.set(event_target_value(&ev))
                ></textarea>
                <span class="field-foot" class:bad=move || !memo_ok()>
                    {move || format!("{} bytes · {}–{} required", memo_len(), MIN_MEMO_BYTES, MAX_MEMO_BYTES)}
                </span>
            </label>

            <div class="field">
                <span class="field-label">"You receive"</span>
                <div class="receive">
                    <span class="big-number">
                        {move || state.supply.get().map(|s| fmt_units(curve::reward_at(s))).unwrap_or("…".into())}
                    </span>
                    <span class="token-chip"><img src="/public/memo-logo.png" alt=""/>"MEMO"</span>
                </div>
                <span class="field-foot">"per mint at the current supply"</span>
            </div>

            <Show when=low_xnt>
                <p class="msg msg-warn">"You need a little XNT in this wallet to pay the network fee."</p>
            </Show>

            <button class="btn-primary btn-wide"
                disabled=move || busy() || (state.wallet.get().is_some() && !memo_ok())
                on:click=mint
            >
                {move || match (status.get(), state.wallet.get()) {
                    (MintStatus::Working(step), _) => step.to_string(),
                    (_, None) => "Connect wallet".to_string(),
                    _ => "Mint".to_string(),
                }}
            </button>

            {move || match status.get() {
                MintStatus::Done { signature, reward, network } => view! {
                    <p class="msg msg-ok">
                        {format!("Minted {} MEMO. ", fmt_units(reward))}
                        <a href=network.tx_url(&signature) target="_blank" rel="noopener">"View transaction ↗"</a>
                    </p>
                }.into_view(),
                MintStatus::Failed(e) => view! { <p class="msg msg-error">{e}</p> }.into_view(),
                _ => ().into_view(),
            }}

            <p class="miner-line">
                "Want to mine automatically? "
                {match MINER_URL {
                    Some(url) => view! { <a href=url target="_blank" rel="noopener">"Download MEMO Miner ↗"</a> }.into_view(),
                    None => view! { <span class="soon">"MEMO Miner"<em>"coming soon"</em></span> }.into_view(),
                }}
            </p>
        </section>
    }
}
