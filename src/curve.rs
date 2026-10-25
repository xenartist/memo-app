//! The memo-mint emission curve (amounts in units, 1 MEMO = 1,000,000 units).
//!
//! Up to and including 10M supply every mint pays 1 MEMO. Above it, supply in (d * 10^k, (d+1) * 10^k]
//! inside the decade (10M * 10^n, 10M * 10^(n+1)] pays decade_base * (10 - d) / 10,
//! decade_base = 1 MEMO / 10^n: above 10M: 0.9, ..., above 90M: 0.1, above 100M: 0.09, ...
//! down to 1 unit above 9T. Minting stops at the 10T cap. Same arithmetic as the contract.

pub const UNITS_PER_TOKEN: u64 = 1_000_000;
const CURVE_START: u64 = 10_000_000 * UNITS_PER_TOKEN;
const BASE_MINT: u64 = UNITS_PER_TOKEN;
pub const MAX_SUPPLY: u64 = 10_000_000_000_000 * UNITS_PER_TOKEN;

/// A mint at supply in (start, end] pays `reward`
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub start: u64,
    pub end: u64,
    pub reward: u64,
}

/// The flat start (0, 10M], then one stage per decade, each split into 9 equal steps
#[derive(Debug, Clone, PartialEq)]
pub struct Stage {
    pub start: u64,
    pub end: u64,
    pub steps: Vec<Step>,
}

/// Exact reward in units for a mint at this supply (0 once the cap is reached)
pub fn reward_at(supply: u64) -> u64 {
    if supply >= MAX_SUPPLY {
        return 0;
    }
    let amount = if supply <= CURVE_START {
        BASE_MINT
    } else {
        let below = supply - 1;
        let mut magnitude = CURVE_START;
        let mut decade_base = BASE_MINT;
        while below / 10 >= magnitude {
            magnitude *= 10;
            decade_base /= 10;
        }
        decade_base * (10 - below / magnitude) / 10
    };
    if supply.saturating_add(amount) > MAX_SUPPLY { 0 } else { amount }
}

pub fn stages() -> Vec<Stage> {
    let mut stages = vec![Stage {
        start: 0,
        end: CURVE_START,
        steps: vec![Step { start: 0, end: CURVE_START, reward: BASE_MINT }],
    }];
    let mut magnitude = CURVE_START;
    let mut decade_base = BASE_MINT;
    while magnitude < MAX_SUPPLY {
        let steps = (1..=9u64)
            .map(|d| Step { start: d * magnitude, end: (d + 1) * magnitude, reward: decade_base * (10 - d) / 10 })
            .collect();
        stages.push(Stage { start: magnitude, end: magnitude * 10, steps });
        magnitude *= 10;
        decade_base /= 10;
    }
    stages
}

/// (stage, step) that a mint at this supply falls in
pub fn locate(supply: u64) -> (usize, usize) {
    let stages = stages();
    for (i, stage) in stages.iter().enumerate() {
        if let Some(j) = stage.steps.iter().position(|s| supply <= s.end) {
            return (i, j);
        }
    }
    (stages.len() - 1, 8)
}

/// Position along the chart, 0.0 to 1.0: stages are equally wide, supply is linear inside a stage
pub fn position(supply: u64) -> f64 {
    let stages = stages();
    let (i, _) = locate(supply);
    let stage = &stages[i];
    let ratio = supply.min(stage.end).saturating_sub(stage.start) as f64 / (stage.end - stage.start) as f64;
    (i as f64 + ratio) / stages.len() as f64
}

/// The next reduction: (supply it takes effect above, reward after it); None at the last step
pub fn next_reduction(supply: u64) -> Option<(u64, u64)> {
    stages().iter().flat_map(|s| s.steps.iter()).find(|s| s.end >= supply).and_then(|s| {
        let after = reward_at(s.end + 1);
        (s.end < MAX_SUPPLY && after > 0).then_some((s.end, after))
    })
}

/// Exact amount from units, trailing zeros trimmed: 900000 -> "0.9"
pub fn fmt_units(units: u64) -> String {
    let (whole, frac) = (units / UNITS_PER_TOKEN, units % UNITS_PER_TOKEN);
    let whole = group(whole);
    if frac == 0 {
        whole
    } else {
        format!("{}.{}", whole, format!("{:06}", frac).trim_end_matches('0'))
    }
}

/// Amount with at most `decimals` decimals: 9082302.685432 -> "9,082,302.68"
pub fn fmt_units_short(units: u64, decimals: u32) -> String {
    let step = UNITS_PER_TOKEN / 10u64.pow(decimals.min(6));
    fmt_units(units / step * step)
}

/// Round supply as a short label: 10M, 200M, 1B, 10T
pub fn fmt_supply_short(units: u64) -> String {
    let tokens = units / UNITS_PER_TOKEN;
    for (div, suffix) in [(1_000_000_000_000u64, "T"), (1_000_000_000, "B"), (1_000_000, "M")] {
        if tokens >= div && tokens % div == 0 {
            return format!("{}{}", tokens / div, suffix);
        }
    }
    group(tokens)
}

/// 1234567 -> "1,234,567"
pub fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
