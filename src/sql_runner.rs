//! SQL query execution and trivia game logic
use rusqlite::{types::Value, Connection, Result};
use std::io::{self, Write};

/// Path to the SQLite database file
pub const DB_PATH: &str = "nfl.sqlite";

/// Result of a completed trivia round containing score and total answers in the questions
pub struct TriviaResult {
    pub score: u32,
    pub total: usize,
}

/// Runs an interactive trivia game where users guess hidden player names.
///
/// Players have 3 strikes. Scoring is out of 1000 points, with harder answers
/// (lower stats) worth more points. The first column should be the player name,
/// and the last column should be the numeric stat for scoring.
pub fn run_trivia(question: &str, sql: &str) -> Result<TriviaResult> {
    let conn = Connection::open(DB_PATH)?;
    let mut stmt = conn.prepare(sql)?;

    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();

    // Fetch all rows into memory
    let rows_iter = stmt.query_map([], |row| {
        let mut vals = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let v: Value = row.get(i)?;
            let s = match v {
                Value::Null => "NULL".to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Real(f) => f.to_string(),
                Value::Text(t) => t,
                Value::Blob(_) => "<blob>".to_string(),
            };
            vals.push(s);
        }
        Ok(vals)
    })?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for row_res in rows_iter {
        rows.push(row_res?);
    }

    if rows.is_empty() {
        println!("(No rows returned for this question.)");
        return Ok(TriviaResult { score: 0, total: 0 });
    }

    let answer_col: usize = 0;
    let total = rows.len();
    let mut guessed = vec![false; total];
    let mut correct = 0usize;
    let mut strikes = 0usize;
    let mut score = 0u32;

    // Calculate point values for each answer
    let point_values = calculate_point_values(&rows, &column_names);

    println!("--- TRIVIA ---");
    println!("{}", &question);
    println!("Guess the hidden names! You have 3 strikes.");
    println!("(Type a player name, e.g. 'Rudolph' or 'Mason Rudolph'. Type 'reveal' to give up.)");
    println!();

    let stdin = io::stdin();

    loop {
        if correct == total || strikes >= 3 {
            break;
        }

        println!("\nQuestion: {}", question);
        println!("--- CURRENT BOARD ---");
        if !column_names.is_empty() {
            println!("{}", column_names.join(" | "));
            println!("{}", "-".repeat(column_names.join(" | ").len()));
        }

        for (i, row) in rows.iter().enumerate() {
            let display_cols: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(j, val)| {
                    if j == answer_col && !guessed[i] {
                        "-------".to_string()
                    } else {
                        val.clone()
                    }
                })
                .collect();

            println!("{:>2}: {}", i + 1, display_cols.join(" | "));
        }

        println!(
            "Correct: {}/{}  Strikes: {}/3  Score: {}",
            correct, total, strikes, score
        );
        println!();

        print!("Enter guess: ");
        io::stdout().flush().ok();

        let mut guess = String::new();
        if stdin.read_line(&mut guess).is_err() {
            println!("Error reading input, try again.");
            continue;
        }
        let guess = guess.trim();
        if guess.is_empty() {
            continue;
        }

        if guess.eq_ignore_ascii_case("reveal") {
            break;
        }

        let guess_lc = guess.to_lowercase();

        // Check if already guessed
        let mut already_got = false;
        for (i, row) in rows.iter().enumerate() {
            let ans_lc = row[answer_col].to_lowercase();
            if ans_lc.contains(&guess_lc) || guess_lc.contains(&ans_lc) {
                if guessed[i] {
                    already_got = true;
                    break;
                }
            }
        }
        if already_got {
            println!("You already got that one!");
            println!();
            continue;
        }

        // Try to match
        let mut found_idx: Option<usize> = None;
        for (i, row) in rows.iter().enumerate() {
            if guessed[i] {
                continue;
            }
            let ans_lc = row[answer_col].to_lowercase();
            if ans_lc.contains(&guess_lc) || guess_lc.contains(&ans_lc) {
                found_idx = Some(i);
                break;
            }
        }

        if let Some(i) = found_idx {
            guessed[i] = true;
            correct += 1;
            let points = point_values[i];
            score += points;
            println!("Correct! {} (+{} points)", rows[i][answer_col], points);
        } else {
            strikes += 1;
            println!("Strike {}!", strikes);
        }
        println!();
    }

    // Print full board
    println!("--- FINAL ANSWERS ---");
    if !column_names.is_empty() {
        println!("{}", column_names.join(" | "));
        println!("{}", "-".repeat(column_names.join(" | ").len()));
    }
    for (i, row) in rows.iter().enumerate() {
        let status = if guessed[i] { "✓" } else { "✗" };
        println!(
            "{:>2} {}: {} ({}pts)",
            i + 1,
            status,
            row.join(" | "),
            point_values[i]
        );
    }
    if correct == total {
        println!("Perfect! You got all {} answers!", total);
    } else if strikes >= 3 {
        println!("Three strikes, you're out!");
    } else {
        println!("Stopping early. Here are the full answers:");
    }
    println!("Final Score: {}/1000", score);
    println!("--- END ---\n");

    Ok(TriviaResult { score, total })
}

/// Calculates point values for each answer based on inverse stat weighting.
///
/// Lower stats = higher points. Equal stats = equal points.
fn calculate_point_values(rows: &[Vec<String>], _column_names: &[String]) -> Vec<u32> {
    let total = rows.len();

    if rows.is_empty() {
        return vec![100; total];
    }

    // The stat column is always in the last column
    let stat_col_idx = rows[0].len() - 1;

    // Parse stat values
    let stats: Vec<f64> = rows
        .iter()
        .filter_map(|row| {
            if row.len() > stat_col_idx {
                row[stat_col_idx].parse::<f64>().ok()
            } else {
                None
            }
        })
        .collect();

    if stats.is_empty() || stats.len() != total {
        // Fallback to equal weight
        let points_each = 1000 / total as u32;
        return vec![points_each; total];
    }

    // Check if all stats are the same (e.g., all have 1 TD)
    let all_same = stats.iter().all(|&s| (s - stats[0]).abs() < 0.01);
    if all_same {
        let points_each = 1000 / total as u32;
        return vec![points_each; total];
    }

    // Inverse scoring: lower stats = higher points
    let max_stat = stats.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_stat = stats.iter().cloned().fold(f64::INFINITY, f64::min);

    let inverses: Vec<f64> = if (max_stat - min_stat).abs() < 0.01 {
        // If all same, equal weight
        vec![1.0; total]
    } else {
        stats.iter().map(|&s| max_stat - s + min_stat).collect()
    };

    // Normalize to sum to 1000
    let sum: f64 = inverses.iter().sum();
    let point_values: Vec<u32> = inverses
        .iter()
        .map(|&inv| ((inv / sum) * 1000.0).round() as u32)
        .collect();

    point_values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_point_distribution() {
        // Test with equal stats (all should get equal points)
        let rows = vec![
            vec!["Player1".to_string(), "100".to_string()],
            vec!["Player2".to_string(), "100".to_string()],
            vec!["Player3".to_string(), "100".to_string()],
        ];
        let column_names = vec!["name".to_string(), "yards".to_string()];

        let points = calculate_point_values(&rows, &column_names);

        assert_eq!(points.len(), 3);
        assert_eq!(points[0], 333); // 1000/3 ≈ 333
        assert_eq!(points[1], 333);
        assert_eq!(points[2], 333);
    }

    #[test]
    fn test_inverse_scoring() {
        // Lower stats should get more points
        let rows = vec![
            vec!["Player1".to_string(), "1000".to_string()],
            vec!["Player2".to_string(), "500".to_string()],
        ];
        let column_names = vec!["name".to_string(), "yards".to_string()];

        let points = calculate_point_values(&rows, &column_names);

        assert_eq!(points.len(), 2);
        // Player with 500 yards should get more points than player with 1000
        assert!(points[1] > points[0]);
    }

    #[test]
    fn test_point_sum_equals_1000() {
        let rows = vec![
            vec!["Player1".to_string(), "800".to_string()],
            vec!["Player2".to_string(), "600".to_string()],
            vec!["Player3".to_string(), "400".to_string()],
        ];
        let column_names = vec!["name".to_string(), "yards".to_string()];

        let points = calculate_point_values(&rows, &column_names);
        let sum: u32 = points.iter().sum();

        // Should sum to approximately 1000 (within rounding)
        assert!((sum as i32 - 1000).abs() <= 2);
    }
}
