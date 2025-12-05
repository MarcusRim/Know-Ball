mod questions;
mod sql_runner;

use crate::questions::{
    build_registry, choose_random_question, generate_sql_for_kind, parse_query,
};
use std::io::{self, Write};

fn main() {
    let registry = build_registry();
    let mut session_score = 0u32;
    let mut questions_played = 0u32;

    println!("Welcome to Know Ball (Rust / SQLite edition)");
    println!("Commands:");
    println!("  start  -> random question");
    println!("  list   -> show all question codes");
    println!("  score  -> show session score");
    println!("  <code> -> run a specific question (e.g., recyds_TEAM_yearrange)");
    println!("  quit   -> exit");
    println!();

    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            eprintln!("Error reading input, try again.");
            continue;
        }

        let raw = input.trim().to_string();
        if raw.is_empty() {
            continue;
        }

        let lc_cmd = raw.to_lowercase();

        match lc_cmd.as_str() {
            "quit" | "exit" => {
                println!("\n=== SESSION SUMMARY ===");
                println!("Questions played: {}", questions_played);
                println!("Total score: {}/{}", session_score, questions_played * 1000);
                if questions_played > 0 {
                    let avg = session_score as f64 / questions_played as f64;
                    println!("Average: {:.1}/1000", avg);
                }
                println!("Goodbye!");
                break;
            }
            "score" => {
                println!("\n=== SESSION SCORE ===");
                println!("Questions played: {}", questions_played);
                println!("Total score: {}/{}", session_score, questions_played * 1000);
                if questions_played > 0 {
                    let avg = session_score as f64 / questions_played as f64;
                    println!("Average: {:.1}/1000", avg);
                }
                println!();
            }
            "list" => {
                println!("Available question codes:");
                let mut codes: Vec<_> = registry.iter().collect();
                codes.sort_by_key(|(code, _)| *code);
                for (code, meta) in codes {
                    println!(" - {code}: {}", meta.description);
                }
                println!();
            }
            "start" => match choose_random_question(&registry) {
                Some((code, meta)) => {
                    println!("Random code: {code}");
                    println!("Description: {}", meta.description);
                    let (q_text, sql) = generate_sql_for_kind(meta.kind, None);
                    println!("Question: {q_text}");

                    match sql_runner::run_trivia(&q_text, &sql) {
                        Ok(result) => {
                            if result.total > 0 {
                                session_score += result.score;
                                questions_played += 1;
                            }
                        }
                        Err(e) => eprintln!("Error running SQL: {e}"),
                    }
                }
                None => {
                    println!("No questions registered.");
                }
            },
            other => {
                // Try team-aware parser
                if let Some(parsed) = parse_query(&raw, &registry) {
                    println!("Code: {raw}");
                    if let Some(ref team) = parsed.team {
                        println!("Team: {team}");
                    }

                    let (q_text, sql) = generate_sql_for_kind(parsed.kind, parsed.team.as_deref());
                    println!("Question: {q_text}");

                    match sql_runner::run_trivia(&q_text, &sql) {
                        Ok(result) => {
                            if result.total > 0 {
                                session_score += result.score;
                                questions_played += 1;
                            }
                        }
                        Err(e) => eprintln!("Error running SQL: {e}"),
                    }
                    continue;
                }

                // Fallback to registry lookup
                let matched = registry
                    .iter()
                    .find(|(k, _)| k.to_ascii_lowercase() == other);

                if let Some((canon_key, meta)) = matched {
                    println!("Code: {canon_key}");
                    println!("Description: {}", meta.description);
                    let (q_text, sql) = generate_sql_for_kind(meta.kind, None);
                    println!("Question: {q_text}");

                    match sql_runner::run_trivia(&q_text, &sql) {
                        Ok(result) => {
                            if result.total > 0 {
                                session_score += result.score;
                                questions_played += 1;
                            }
                        }
                        Err(e) => eprintln!("Error running SQL: {e}"),
                    }
                } else {
                    println!("Unknown command or code: '{other}'");
                    println!("Type 'list' to see available codes.\n");
                }
            }
        }
    }
}
