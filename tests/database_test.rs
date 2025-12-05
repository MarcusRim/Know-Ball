use rusqlite::Connection;

const DB_PATH: &str = "nfl.sqlite";

#[test]
fn test_database_exists_and_opens() {
    let conn = Connection::open(DB_PATH);
    assert!(
        conn.is_ok(),
        "Database file should exist and open successfully"
    );
}

#[test]
fn test_players_table_exists() {
    let conn = Connection::open(DB_PATH).unwrap();
    let result = conn.query_row(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='players'",
        [],
        |row| row.get::<_, String>(0),
    );
    assert!(result.is_ok(), "players table should exist");
}

#[test]
fn test_seasons_table_exists() {
    let conn = Connection::open(DB_PATH).unwrap();
    let result = conn.query_row(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='seasons'",
        [],
        |row| row.get::<_, String>(0),
    );
    assert!(result.is_ok(), "seasons table should exist");
}

#[test]
fn test_players_table_has_data() {
    let conn = Connection::open(DB_PATH).unwrap();
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))
        .unwrap();
    assert!(count > 0, "players table should have data");
}

#[test]
fn test_seasons_table_has_data() {
    let conn = Connection::open(DB_PATH).unwrap();
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM seasons", [], |row| row.get(0))
        .unwrap();
    assert!(count > 0, "seasons table should have data");
}

// MANUAL VALIDATION TEST - You need to fill in the expected values
#[test]
fn test_last10passers_pit_specific_results() {
    let conn = Connection::open(DB_PATH).unwrap();

    // Match the ACTUAL SQL from your questions.rs
    let sql = "WITH latest AS (
            SELECT s.player_id, s.team_abbr, s.season, s.attempts,
                   ROW_NUMBER() OVER (PARTITION BY s.player_id ORDER BY s.season DESC, s.attempts DESC) as rn
            FROM seasons s
            WHERE s.team_abbr = 'PIT' AND s.attempts >= 10
        )
        SELECT p.name, latest.team_abbr, latest.season, latest.attempts
        FROM latest
        JOIN players p ON p.player_id = latest.player_id
        WHERE latest.rn = 1
        ORDER BY latest.season DESC, latest.attempts DESC
        LIMIT 10";

    let mut stmt = conn.prepare(sql).unwrap();
    let results: Vec<(String, String, i32, i32)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), 10);

    assert_eq!(
        results[0],
        ("Russell Wilson".to_string(), "PIT".to_string(), 2024, 336)
    );
    assert_eq!(
        results[1],
        ("Justin Fields".to_string(), "PIT".to_string(), 2024, 161)
    );
    assert_eq!(
        results[2],
        ("Kenny Pickett".to_string(), "PIT".to_string(), 2023, 324)
    );
    assert_eq!(
        results[3],
        (
            "Mitchell Trubisky".to_string(),
            "PIT".to_string(),
            2023,
            107
        )
    );
    assert_eq!(
        results[4],
        ("Mason Rudolph".to_string(), "PIT".to_string(), 2023, 74)
    );
    assert_eq!(
        results[5],
        (
            "Ben Roethlisberger".to_string(),
            "PIT".to_string(),
            2021,
            605
        )
    );
    assert_eq!(
        results[6],
        ("Devlin Hodges".to_string(), "PIT".to_string(), 2019, 160)
    );
    assert_eq!(
        results[7],
        ("Joshua Dobbs".to_string(), "PIT".to_string(), 2018, 12)
    );
    assert_eq!(
        results[8],
        ("Landry Jones".to_string(), "PIT".to_string(), 2017, 28)
    );
    assert_eq!(
        results[9],
        ("Michael Vick".to_string(), "PIT".to_string(), 2015, 66)
    );

    // Verify all are PIT
    for (_, team, _, _) in &results {
        assert_eq!(team, "PIT");
    }

    println!("\n=== ACTUAL RESULTS FOR last10passers_PIT ===");
    for (i, (name, team, season, attempts)) in results.iter().enumerate() {
        println!(
            "{}: {} | {} | {} | {} attempts",
            i + 1,
            name,
            team,
            season,
            attempts
        );
    }
    println!("=== Copy these values into your test assertions ===\n");
}

// MANUAL VALIDATION TEST - For scoring calculation with team-specific question
#[test]
fn test_top10passers_tb_with_scoring() {
    let conn = Connection::open(DB_PATH).unwrap();

    // This matches passyds_TEAM for TB (since 2000)
    let sql = "SELECT p.name,
                (SELECT s2.team_abbr
                 FROM seasons s2
                 WHERE s2.player_id = s.player_id
                   AND s2.team_abbr = 'TB'
                 ORDER BY s2.season DESC
                 LIMIT 1) AS last_team,
                SUM(s.passing_yards) AS pass_yards
         FROM seasons s
         JOIN players p ON p.player_id = s.player_id
         WHERE s.team_abbr = 'TB' AND s.season >= 2000
         GROUP BY s.player_id
         ORDER BY pass_yards DESC
         LIMIT 10";

    let mut stmt = conn.prepare(sql).unwrap();
    let results: Vec<(String, String, i32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), 10, "Should return exactly 10 results");

    // Test each result
    assert_eq!(
        results[0],
        ("Jameis Winston".to_string(), "TB".to_string(), 19737)
    );
    assert_eq!(
        results[1],
        ("Tom Brady".to_string(), "TB".to_string(), 14643)
    );
    assert_eq!(
        results[2],
        ("Josh Freeman".to_string(), "TB".to_string(), 13726)
    );
    assert_eq!(
        results[3],
        ("Brad Johnson".to_string(), "TB".to_string(), 10950)
    );
    assert_eq!(
        results[4],
        ("Baker Mayfield".to_string(), "TB".to_string(), 8544)
    );
    assert_eq!(
        results[5],
        ("Jeff Garcia".to_string(), "TB".to_string(), 5152)
    );
    assert_eq!(
        results[6],
        ("Brian Griese".to_string(), "TB".to_string(), 4841)
    );
    assert_eq!(
        results[7],
        ("Mike Glennon".to_string(), "TB".to_string(), 4100)
    );
    assert_eq!(
        results[8],
        ("Ryan Fitzpatrick".to_string(), "TB".to_string(), 3469)
    );
    assert_eq!(
        results[9],
        ("Shaun King".to_string(), "TB".to_string(), 3109)
    );

    // Verify descending order of passing yards
    for i in 0..results.len() - 1 {
        assert!(
            results[i].2 >= results[i + 1].2,
            "Passing yards should be in descending order"
        );
    }

    // Calculate expected point values (inverse scoring)
    let yards: Vec<f64> = results.iter().map(|r| r.2 as f64).collect();
    let max_yards = yards.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_yards = yards.iter().cloned().fold(f64::INFINITY, f64::min);

    let inverses: Vec<f64> = yards.iter().map(|&y| max_yards - y + min_yards).collect();
    let sum: f64 = inverses.iter().sum();
    let point_values: Vec<u32> = inverses
        .iter()
        .map(|&inv| ((inv / sum) * 1000.0).round() as u32)
        .collect();

    // Test point values match expected
    assert_eq!(point_values[0], 22);
    assert_eq!(point_values[1], 59);
    assert_eq!(point_values[2], 65);
    assert_eq!(point_values[3], 85);
    assert_eq!(point_values[4], 102);
    assert_eq!(point_values[5], 126);
    assert_eq!(point_values[6], 128);
    assert_eq!(point_values[7], 134);
    assert_eq!(point_values[8], 138);
    assert_eq!(point_values[9], 141);

    // Verify point values sum to approximately 1000
    let total_points: u32 = point_values.iter().sum();
    assert!(
        (total_points as i32 - 1000).abs() <= 2,
        "Points should sum to ~1000"
    );

    // Verify inverse scoring: lower yards = higher points
    assert!(
        point_values[9] > point_values[0],
        "Last place should have more points than first"
    );
}
