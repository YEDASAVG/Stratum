//! Compares the rules classifier against Jev.
//!
//! Needs TYPESAFE_API_KEY in .env or the environment.
//!   cargo run -p logai-rag --example jev_smoke

use std::time::Instant;

use logai_rag::{Classifier, JevClient, QueryAnalyzer};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let jev = match JevClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "\n  {e}\n\n  Set it in one of these, then retry:\n    \
                 .env at the repo root:  TYPESAFE_API_KEY=ts-...\n    \
                 this shell:             export TYPESAFE_API_KEY='ts-...'\n\n  \
                 Get a key: https://console.typesafe.ai/settings/keys\n"
            );
            std::process::exit(1);
        }
    };

    // Stands in for SELECT DISTINCT service FROM logs. None of these match
    // the 17 names hardcoded in extract_service.
    let services: Vec<String> = [
        "payments-api",
        "ledger-worker",
        "cart-svc",
        "auth-gateway",
        "notification-dispatcher",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let queries = [
        "why did payments-api start throwing 500s at 3am",
        "anything weird with the ledger worker in the last hour",
        "walk me through what happened to request 7f3a92",
        "give me the gist of last night",
        "show me errors last 2 hours",
        "checkout is timing out for some people",
    ];

    println!("\n  model: {}", jev.model());

    // Without this, a failing API falls back to rules on every row and the
    // table looks like the two agree.
    let preflight = jev
        .system_one(
            &serde_json::json!({ "text": "the database is down" }),
            &serde_json::json!({
                "ok": { "type": "noul", "instructions": "Does `text` describe a problem?" }
            }),
        )
        .await;

    match preflight {
        Ok(r) => println!(
            "  preflight: ok ({} in / {} out tokens)\n",
            r.usage.input_tokens, r.usage.output_tokens
        ),
        Err(e) => {
            eprintln!("\n  PREFLIGHT FAILED\n  {e}\n");
            eprintln!("  Nothing below would be real, so stopping here.\n");
            std::process::exit(1);
        }
    }

    let analyzer = QueryAnalyzer::new();

    println!(
        "  {:<44} {:<26} {:<34}",
        "QUERY", "RULES intent / service", "JEV intent@conf / service@conf"
    );
    println!("  {}", "-".repeat(108));

    let mut total_ms = 0u128;
    let mut answered_by_jev = 0usize;
    let mut disagreements = 0usize;

    for q in queries {
        let rules = analyzer.analyze(q);

        let t0 = Instant::now();
        let jevd = analyzer.analyze_with_jev(q, &jev, &services).await;
        let ms = t0.elapsed().as_millis();
        total_ms += ms;

        let rules_col = format!(
            "{:?} / {}",
            rules.intent,
            rules.service.as_deref().unwrap_or("-")
        );
        let jev_col = format!(
            "{:?}@{:.2} / {}@{:.2}",
            jevd.intent,
            jevd.intent_confidence.unwrap_or(0.0),
            jevd.service.as_deref().unwrap_or("-"),
            jevd.service_confidence.unwrap_or(0.0)
        );

        let via_jev = jevd.classified_by == Classifier::Jev;
        if via_jev {
            answered_by_jev += 1;
        }
        let differs = rules.intent != jevd.intent || rules.service != jevd.service;
        if differs {
            disagreements += 1;
        }
        let mark = if !via_jev {
            "FELL BACK TO RULES"
        } else if differs {
            "<- differs"
        } else {
            ""
        };

        println!(
            "  {:<44.44} {:<26.26} {:<34.34} [{:>4}ms] {}",
            q, rules_col, jev_col, ms, mark
        );

        assert_eq!(rules.from.is_some(), jevd.from.is_some());
        assert!(matches!(jevd.classified_by, Classifier::Jev | Classifier::Rules));
    }

    let n = queries.len();
    println!(
        "\n  {n} queries | {answered_by_jev}/{n} answered by Jev | \
         {disagreements} disagreement(s) | {}ms avg",
        total_ms / n as u128
    );

    if answered_by_jev == 0 {
        eprintln!(
            "\n  Every row fell back to the rules path. The Jev column above is\n  \
             a copy of the rules column, not a second opinion.\n"
        );
        std::process::exit(1);
    }
    if disagreements == 0 {
        println!(
            "\n  Jev agreed with the regex on all {n}. Either the queries are too\n  \
             easy, or the confidence floors are too high to ever override.\n"
        );
    } else {
        println!("\n  Rows marked '<- differs' are where the regex was wrong.\n");
    }
}
