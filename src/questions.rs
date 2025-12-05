//! NFL trivia question types, SQL generation, and question registry.
//!
//! This module defines all available trivia questions, handles random parameter
//! generation (teams, years, year ranges), and generates corresponding SQL queries.
use rand::seq::{IteratorRandom, SliceRandom};
use rand::Rng;
use std::collections::HashMap;

/// Starting year for data (2000)
pub const START_YEAR: i32 = 2000;

/// Ending year for data (2024)
pub const END_YEAR: i32 = 2024;

/// All 32 NFL team abbreviations
pub const TEAMS: [&str; 32] = [
    "BUF", "MIA", "NE", "NYJ", "BAL", "CIN", "CLE", "PIT", "HOU", "IND", "JAX", "TEN", "DEN", "KC",
    "LV", "LAC", "DAL", "NYG", "PHI", "WAS", "CHI", "DET", "GB", "MIN", "ATL", "CAR", "NO", "TB",
    "ARI", "LAR", "SF", "SEA",
];

/// Types of trivia questions available
#[derive(Debug, Clone, Copy)]
pub enum QuestionKind {
    RecYdsTeamYearRange,
    RushYdsTeamYearRange,
    PassYdsTeamSinceStart,
    Last10PassersTeam,
    Last10RushersTeam,
    Last10ReceiversTeam,
    Last10IntThrowersTeam,
    Last10TdPassersTeam,
    Last10NonQbPassersTeam,
    Last10MidWrsTeam,
    Last10MidRbsTeam,
    Top10FumblesLostYearRange,
    Top10RushTdYearRange,
    Top10RecTdYearRange,
    Top10PassTdYearRange,
    Top10IntThrownYearRange,
    Top10RushingQbYearRange,
    Top10ReceivingTeYearRange,
    Top10ReceivingRbYearRange,
    Top10RushingWrYearRange,
    Top10ReceptionsYearRange,
    Top10CompPercYear,
    Top10PassYdsYear,
    Top10YpcYear,
    Top10YprYear,
    Top10RushersYear,
    Top10ReceiversYear,
    Top10RushingQbYear,
    Top10ReceivingTeYear,
}

/// Metadata for a question type including description and kind
#[derive(Debug, Clone, Copy)]
pub struct QuestionMeta {
    pub description: &'static str,
    pub kind: QuestionKind,
}

/// Selects a random team
fn random_team<R: Rng + ?Sized>(rng: &mut R) -> &'static str {
    TEAMS.choose(rng).copied().unwrap()
}

/// Selects a random year between START_YEAR and END_YEAR (inclusive)
fn random_year<R: Rng + ?Sized>(rng: &mut R) -> i32 {
    rng.gen_range(START_YEAR..=END_YEAR)
}

/// Selects a random year range between START_YEAR and END_YEAR (inclusive)
fn random_year_range<R: Rng + ?Sized>(rng: &mut R) -> (i32, i32) {
    // inclusive, at least 2 years long
    let start = rng.gen_range(START_YEAR..END_YEAR);
    let end = rng.gen_range((start + 1)..=END_YEAR);
    (start, end)
}

// Parsed user request containing question kind and optional team filter
pub struct ParsedRequest {
    pub kind: QuestionKind,
    pub team: Option<String>,
}

/// Parses user input to extract question kind and team (if specified).
///
/// Supports inputs like "last10rushers_PIT" where PIT is the team code.
pub fn parse_query(input: &str, registry: &HashMap<String, QuestionMeta>) -> Option<ParsedRequest> {
    let raw = input.trim();

    // Split into parts on underscore
    let parts: Vec<&str> = raw.split('_').collect();
    if parts.is_empty() {
        return None;
    }

    // Check if last part is a valid team code
    let last = parts.last().unwrap().to_ascii_uppercase();
    let team = if TEAMS.iter().any(|&code| code == last) {
        Some(last)
    } else {
        None
    };

    // Extract base code without team suffix
    let base = if team.is_some() {
        parts[..parts.len() - 1].join("_")
    } else {
        raw.to_string()
    };

    let mut candidates: Vec<String> = Vec::new();
    let base_lower = base.to_ascii_lowercase();
    candidates.push(base_lower.clone());
    if team.is_some() {
        candidates.push(format!("{}_team", base_lower));
    }

    let found = registry.iter().find(|(k, _)| {
        let key_lower = k.to_ascii_lowercase();
        candidates.iter().any(|c| c == &key_lower)
    })?;

    let (_, meta) = found;

    Some(ParsedRequest {
        kind: meta.kind,
        team,
    })
}

/// Builds registry mapping question codes to their metadata
pub fn build_registry() -> HashMap<String, QuestionMeta> {
    let mut m = HashMap::new();

    fn add(
        m: &mut HashMap<String, QuestionMeta>,
        code: &str,
        desc: &'static str,
        kind: QuestionKind,
    ) {
        m.insert(
            code.to_string(),
            QuestionMeta {
                description: desc,
                kind,
            },
        );
    }

    // --- team + year range ---
    add(
        &mut m,
        "recyds_yearrange_TEAM",
        "Top 10 receiving yards for a team in a year range",
        QuestionKind::RecYdsTeamYearRange,
    );
    add(
        &mut m,
        "rushyds_yearrange_TEAM",
        "Top 10 rushing yards for a team in a year range",
        QuestionKind::RushYdsTeamYearRange,
    );
    add(
        &mut m,
        "passyds_TEAM",
        "Top 10 passing yards for a team since the start year",
        QuestionKind::PassYdsTeamSinceStart,
    );

    // --- last-10 style ---
    add(
        &mut m,
        "last10passers_TEAM",
        "Last 10 players to attempt at least 10 passes for a team",
        QuestionKind::Last10PassersTeam,
    );
    add(
        &mut m,
        "last10rushers_TEAM",
        "Last 10 non-QBs to attempt at least 30 rushes for a team",
        QuestionKind::Last10RushersTeam,
    );
    add(
        &mut m,
        "last10receivers_TEAM",
        "Last 10 players to record at least 20 receptions for a team",
        QuestionKind::Last10ReceiversTeam,
    );
    add(
        &mut m,
        "last10intthrowers_TEAM",
        "Last 10 players to throw an interception for a team",
        QuestionKind::Last10IntThrowersTeam,
    );
    add(
        &mut m,
        "last10tdpassers_TEAM",
        "Last 10 players to throw a passing TD for a team",
        QuestionKind::Last10TdPassersTeam,
    );
    add(
        &mut m,
        "last10nonqbp_TEAM",
        "Last 10 non-QBs to attempt a pass for a team",
        QuestionKind::Last10NonQbPassersTeam,
    );
    add(
        &mut m,
        "last10midwrs_TEAM",
        "Last 10 WRs (<3000 career rec yards) to score a rec TD for a team",
        QuestionKind::Last10MidWrsTeam,
    );
    add(
        &mut m,
        "last10midrbs_TEAM",
        "Last 10 RBs (<3000 career rush yards) to score a rush TD for a team",
        QuestionKind::Last10MidRbsTeam,
    );

    // --- year range global ---
    add(
        &mut m,
        "top10fumlost_yearrange",
        "Top 10 players with most fumbles lost in a year range",
        QuestionKind::Top10FumblesLostYearRange,
    );
    add(
        &mut m,
        "top10rushtd_yearrange",
        "Top 10 players with most rushing TDs in a year range",
        QuestionKind::Top10RushTdYearRange,
    );
    add(
        &mut m,
        "top10rectd_yearrange",
        "Top 10 players with most receiving TDs in a year range",
        QuestionKind::Top10RecTdYearRange,
    );
    add(
        &mut m,
        "top10passtd_yearrange",
        "Top 10 players with most passing TDs in a year range",
        QuestionKind::Top10PassTdYearRange,
    );
    add(
        &mut m,
        "top10intthrown_yearrange",
        "Top 10 players with most interceptions thrown in a year range",
        QuestionKind::Top10IntThrownYearRange,
    );
    add(
        &mut m,
        "top10rushingqb_yearrange",
        "Top 10 QBs in rushing yards in a year range",
        QuestionKind::Top10RushingQbYearRange,
    );
    add(
        &mut m,
        "top10receivingte_yearrange",
        "Top 10 TEs in receiving yards in a year range",
        QuestionKind::Top10ReceivingTeYearRange,
    );
    add(
        &mut m,
        "top10receivingrb_yearrange",
        "Top 10 RBs in receiving yards in a year range",
        QuestionKind::Top10ReceivingRbYearRange,
    );
    add(
        &mut m,
        "top10rushingwr_yearrange",
        "Top 10 WRs in rushing yards in a year range",
        QuestionKind::Top10RushingWrYearRange,
    );
    add(
        &mut m,
        "top10receptions_yearrange",
        "Top 10 players in receptions in a year range",
        QuestionKind::Top10ReceptionsYearRange,
    );

    // --- single-season ---
    add(
        &mut m,
        "top10compperc_year",
        "Top 10 QBs in completion percentage in one season",
        QuestionKind::Top10CompPercYear,
    );
    add(
        &mut m,
        "top10passyds_year",
        "Top 10 QBs in passing yards in one season",
        QuestionKind::Top10PassYdsYear,
    );
    add(
        &mut m,
        "top10ypc_year",
        "Top 10 rushers in yards per carry in one season",
        QuestionKind::Top10YpcYear,
    );
    add(
        &mut m,
        "top10ypr_year",
        "Top 10 receivers in yards per reception in one season",
        QuestionKind::Top10YprYear,
    );
    add(
        &mut m,
        "top10rushers_year",
        "Top 10 rushers in rushing yards in one season",
        QuestionKind::Top10RushersYear,
    );
    add(
        &mut m,
        "top10receivers_year",
        "Top 10 receivers in receiving yards in one season",
        QuestionKind::Top10ReceiversYear,
    );
    add(
        &mut m,
        "top10rushingqb_year",
        "Top 10 rushing QBs in one season",
        QuestionKind::Top10RushingQbYear,
    );
    add(
        &mut m,
        "top10receivingte_year",
        "Top 10 TEs in receiving yards in one season",
        QuestionKind::Top10ReceivingTeYear,
    );

    m
}

/// Chooses a random question from the registry
pub fn choose_random_question<'a>(
    registry: &'a HashMap<String, QuestionMeta>,
) -> Option<(&'a str, QuestionMeta)> {
    let mut rng = rand::thread_rng();
    registry
        .iter()
        .choose(&mut rng)
        .map(|(code, meta)| (code.as_str(), *meta))
}

/// Generates question text and SQL query for a given question kind.
///
/// Randomly selects parameters (teams, years, year ranges) and constructs
/// the appropriate SQL query.
pub fn generate_sql_for_kind(kind: QuestionKind, team_override: Option<&str>) -> (String, String) {
    let mut rng = rand::thread_rng();

    match kind {
        // ---------------- team + year range ----------------
        QuestionKind::RecYdsTeamYearRange => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players in receiving yards for {team} between {s}–{e}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, SUM(s.receiving_yards) AS rec_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.team_abbr = '{team}' AND s.season BETWEEN {s} AND {e}\n\
                 GROUP BY s.player_id\n\
                 ORDER BY rec_yards DESC\n\
                 LIMIT 10;",
                team = team,
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::RushYdsTeamYearRange => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players in rushing yards for {team} between {s}–{e}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, SUM(s.rushing_yards) AS rush_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.team_abbr = '{team}' AND s.season BETWEEN {s} AND {e}\n\
                 GROUP BY s.player_id\n\
                 ORDER BY rush_yards DESC\n\
                 LIMIT 10;",
                team = team,
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::PassYdsTeamSinceStart => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Top 10 players in passing yards for {team} since {start} (inclusive).",
                start = START_YEAR
            );
            let sql = format!(
                "SELECT p.name, s.team_abbr, SUM(s.passing_yards) AS pass_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.team_abbr = '{team}' AND s.season >= {start}\n\
                 GROUP BY s.player_id\n\
                 ORDER BY pass_yards DESC\n\
                 LIMIT 10;",
                team = team,
                start = START_YEAR,
            );
            (q, sql)
        }

        // ---------------- last-10 style ----------------
        QuestionKind::Last10PassersTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 player-seasons with ≥10 pass attempts for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.attempts\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND attempts >= 10\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.attempts >= 10\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.attempts\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10RushersTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 non-QB player-seasons with ≥30 rush attempts for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.rushing_attempts\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND position <> 'QB' AND rushing_attempts >= 30\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.position <> 'QB' AND s.rushing_attempts >= 30\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.rushing_attempts\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10ReceiversTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 player-seasons with ≥20 receptions for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.receptions\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND receptions >= 20\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.receptions >= 20\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.receptions\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10IntThrowersTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 player-seasons with ≥1 interception thrown for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.interceptions\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND interceptions > 0\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.interceptions > 0\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.interceptions\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10TdPassersTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 player-seasons with ≥3 passing TD for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.passing_tds\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND passing_tds > 2\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.passing_tds > 2\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.passing_tds\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10NonQbPassersTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 non-QB player-seasons with ≥1 pass attempt for {team} (most recent first)."
            );
            let sql = format!(
                "WITH latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.attempts\n\
                    FROM seasons s\n\
                    JOIN (\n\
                        SELECT player_id, MAX(season) AS max_season\n\
                        FROM seasons\n\
                        WHERE team_abbr = '{team}' AND position <> 'QB' AND attempts > 0\n\
                        GROUP BY player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}' AND s.position <> 'QB' AND s.attempts > 0\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.attempts\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10MidWrsTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 WRs (200 < career rec yards < 3000) to score a receiving TD for {team} (most recent first)."
            );
            let sql = format!(
                "WITH career AS (\n\
                    SELECT player_id, SUM(receiving_yards) AS career_rec_yds\n\
                    FROM seasons\n\
                    GROUP BY player_id\n\
                ),\n\
                latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.receiving_tds, career.career_rec_yds\n\
                    FROM seasons s\n\
                    JOIN career ON career.player_id = s.player_id\n\
                    JOIN (\n\
                        SELECT s2.player_id, MAX(s2.season) AS max_season\n\
                        FROM seasons s2\n\
                        JOIN career c2 ON c2.player_id = s2.player_id\n\
                        WHERE s2.team_abbr = '{team}'\n\
                        AND s2.position = 'WR'\n\
                        AND c2.career_rec_yds < 3000\n\
                        AND c2.career_rec_yds > 200\n\
                        AND s2.receiving_tds > 0\n\
                        GROUP BY s2.player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}'\n\
                    AND s.position = 'WR'\n\
                    AND career.career_rec_yds < 3000\n\
                    AND career.career_rec_yds > 200\n\
                    AND s.receiving_tds > 0\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.receiving_tds, latest.career_rec_yds\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        QuestionKind::Last10MidRbsTeam => {
            let team = match team_override {
                Some(t) => t.to_string(),
                None => random_team(&mut rng).to_string(),
            };
            let q = format!(
                "Last 10 RBs (200 < career rush yards < 3000) to score a rushing TD for {team} (most recent first)."
            );
            let sql = format!(
                "WITH career AS (\n\
                    SELECT player_id, SUM(rushing_yards) AS career_rush_yds\n\
                    FROM seasons\n\
                    GROUP BY player_id\n\
                ),\n\
                latest AS (\n\
                    SELECT s.player_id, s.team_abbr, s.season, s.rushing_tds, career.career_rush_yds\n\
                    FROM seasons s\n\
                    JOIN career ON career.player_id = s.player_id\n\
                    JOIN (\n\
                        SELECT s2.player_id, MAX(s2.season) AS max_season\n\
                        FROM seasons s2\n\
                        JOIN career c2 ON c2.player_id = s2.player_id\n\
                        WHERE s2.team_abbr = '{team}'\n\
                        AND s2.position = 'RB'\n\
                        AND c2.career_rush_yds < 3000\n\
                        AND c2.career_rush_yds > 200\n\
                        AND s2.rushing_tds > 0\n\
                        GROUP BY s2.player_id\n\
                    ) m ON m.player_id = s.player_id AND m.max_season = s.season\n\
                    WHERE s.team_abbr = '{team}'\n\
                    AND s.position = 'RB'\n\
                    AND career.career_rush_yds < 3000\n\
                    AND career.career_rush_yds > 200\n\
                    AND s.rushing_tds > 0\n\
                )\n\
                SELECT p.name, latest.team_abbr, latest.season, latest.rushing_tds, latest.career_rush_yds\n\
                FROM latest\n\
                JOIN players p ON p.player_id = latest.player_id\n\
                ORDER BY latest.season DESC\n\
                LIMIT 10;",
                team = team,
            );
            (q, sql)
        }

        // ---------------- year-range globals ----------------
        QuestionKind::Top10FumblesLostYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players with most fumbles lost between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.fumbles_lost) AS fum_lost\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY fum_lost DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10RushTdYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players with most rushing TDs between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.rushing_tds) AS rush_tds\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY rush_tds DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10RecTdYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players with most receiving TDs between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.receiving_tds) AS rec_tds\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY rec_tds DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10PassTdYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players with most passing TDs between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.passing_tds) AS pass_tds\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY pass_tds DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10IntThrownYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players with most interceptions thrown between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.interceptions) AS ints\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY ints DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10RushingQbYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 QBs in rushing yards between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                    AND s2.position = 'QB'\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.rushing_yards) AS rush_yards\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e} AND s.position = 'QB'\n\
                GROUP BY s.player_id\n\
                ORDER BY rush_yards DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10ReceivingTeYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 TEs in receiving yards between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                    AND s2.position = 'TE'\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.receiving_yards) AS rec_yards\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e} AND s.position = 'TE'\n\
                GROUP BY s.player_id\n\
                ORDER BY rec_yards DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10ReceivingRbYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 RBs in receiving yards between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                    AND s2.position = 'RB'\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.receiving_yards) AS rec_yards\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e} AND s.position = 'RB'\n\
                GROUP BY s.player_id\n\
                ORDER BY rec_yards DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10RushingWrYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 WRs in rushing yards between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                    AND s2.position = 'WR'\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.rushing_yards) AS rush_yards\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e} AND s.position = 'WR'\n\
                GROUP BY s.player_id\n\
                ORDER BY rush_yards DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }
        QuestionKind::Top10ReceptionsYearRange => {
            let (s, e) = random_year_range(&mut rng);
            let q = format!("Top 10 players in total receptions between {s}–{e}.");
            let sql = format!(
                "SELECT p.name,\n\
                (SELECT s2.team_abbr\n\
                FROM seasons s2\n\
                WHERE s2.player_id = s.player_id\n\
                    AND s2.season BETWEEN {s} AND {e}\n\
                ORDER BY s2.season DESC\n\
                LIMIT 1) AS last_team,\n\
                SUM(s.receptions) AS recs\n\
                FROM seasons s\n\
                JOIN players p ON p.player_id = s.player_id\n\
                WHERE s.season BETWEEN {s} AND {e}\n\
                GROUP BY s.player_id\n\
                ORDER BY recs DESC\n\
                LIMIT 10;",
                s = s,
                e = e,
            );
            (q, sql)
        }

        // ---------------- SINGLE SEASON ----------------
        QuestionKind::Top10CompPercYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 QBs in completion percentage in {year} (min 100 attempts).");
            let sql = format!(
                "SELECT p.name,\n\
                        s.team_abbr,\n\
                        s.season,\n\
                        s.completions,\n\
                        s.attempts,\n\
                        1.0 * s.completions / s.attempts AS comp_pct\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.position = 'QB' AND s.attempts >= 100\n\
                 ORDER BY comp_pct DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10PassYdsYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 QBs in passing yards in {year}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, s.season, s.passing_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.position = 'QB'\n\
                 ORDER BY s.passing_yards DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10YpcYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 players in yards per carry in {year} (min 50 rush attempts).");
            let sql = format!(
                "SELECT p.name,\n\
                        s.team_abbr,\n\
                        s.season,\n\
                        s.rushing_attempts,\n\
                        s.rushing_yards,\n\
                        1.0 * s.rushing_yards / s.rushing_attempts AS ypc\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.rushing_attempts >= 50\n\
                 ORDER BY ypc DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10YprYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 players in yards per reception in {year} (min 50 targets).");
            let sql = format!(
                "SELECT p.name,\n\
                        s.team_abbr,\n\
                        s.season,\n\
                        s.targets,\n\
                        s.receptions,\n\
                        s.receiving_yards,\n\
                        1.0 * s.receiving_yards / s.receptions AS ypr\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.targets >= 50 AND s.receptions > 0\n\
                 ORDER BY ypr DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10RushersYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 rushers in rushing yards in {year}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, s.season, s.rushing_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year}\n\
                 ORDER BY s.rushing_yards DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10ReceiversYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 pass catchers in receiving yards in {year}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, s.season, s.receiving_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year}\n\
                 ORDER BY s.receiving_yards DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10RushingQbYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 QBs in rushing yards in {year}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, s.season, s.rushing_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.position = 'QB'\n\
                 ORDER BY s.rushing_yards DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
        QuestionKind::Top10ReceivingTeYear => {
            let year = random_year(&mut rng);
            let q = format!("Top 10 TEs in receiving yards in {year}.");
            let sql = format!(
                "SELECT p.name, s.team_abbr, s.season, s.receiving_yards\n\
                 FROM seasons s\n\
                 JOIN players p ON p.player_id = s.player_id\n\
                 WHERE s.season = {year} AND s.position = 'TE'\n\
                 ORDER BY s.receiving_yards DESC\n\
                 LIMIT 10;",
                year = year,
            );
            (q, sql)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_year_in_range() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let year = random_year(&mut rng);
            assert!(year >= START_YEAR && year <= END_YEAR);
        }
    }

    #[test]
    fn test_random_year_range_valid() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let (start, end) = random_year_range(&mut rng);
            assert!(start >= START_YEAR);
            assert!(end <= END_YEAR);
            assert!(end > start); // At least 2 years
            assert!(end >= start + 1);
        }
    }

    #[test]
    fn test_parse_query_with_team() {
        let registry = build_registry();
        let result = parse_query("last10passers_PIT", &registry);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.team, Some("PIT".to_string()));
    }

    #[test]
    fn test_parse_query_without_team() {
        let registry = build_registry();
        let result = parse_query("top10fumlost_yearrange", &registry);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.team, None);
    }

    #[test]
    fn test_parse_query_invalid_team() {
        let registry = build_registry();
        // XYZ is not a valid team
        let result = parse_query("last10passers_XYZ", &registry);

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_query_case_insensitive() {
        let registry = build_registry();
        let result = parse_query("LAST10PASSERS_pit", &registry);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.team, Some("PIT".to_string()));
    }

    #[test]
    fn test_build_registry_not_empty() {
        let registry = build_registry();
        assert!(!registry.is_empty());
        assert!(registry.len() > 20); // Should have lots of questions
    }

    #[test]
    fn test_all_teams_valid() {
        // Make sure all teams in TEAMS array are 2-3 chars
        for team in TEAMS.iter() {
            assert!(team.len() >= 2 && team.len() <= 3);
            assert!(team.chars().all(|c| c.is_ascii_uppercase()));
        }
    }

    #[test]
    fn test_generate_sql_contains_team() {
        let (question, sql) = generate_sql_for_kind(QuestionKind::Last10PassersTeam, Some("IND"));

        assert!(sql.contains("IND"));
        assert!(question.contains("IND"));
    }

    #[test]
    fn test_choose_random_question_returns_valid() {
        let registry = build_registry();
        let result = choose_random_question(&registry);
        assert!(result.is_some());
    }

    #[test]
    fn test_sql_has_order_by_and_limit() {
        // All queries should have ORDER BY and LIMIT
        let (_, sql) = generate_sql_for_kind(QuestionKind::Top10PassYdsYear, None);
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_year_range_questions_have_between() {
        let (_, sql) = generate_sql_for_kind(QuestionKind::Top10RushTdYearRange, None);
        assert!(sql.contains("BETWEEN"));
    }
}
